use std::time::Duration;

use crate::domain::Email;
use crate::email_client::EmailClient;
use crate::{configuration::Settings, startup::get_connection_pool};
use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use tracing::field::debug;
use tracing::{field::display, Span};
use uuid::Uuid;

const MAX_RETRY_DELAY_SEC: u64 = 300;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[derive(thiserror::Error, Debug)]
pub enum WorkerError {
    #[error("Email: {0} has invalid format")]
    InvalidEmailError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Task {
    newsletter_issue_id: Uuid,
    subscriber_email: String,
    n_retries: i16,
    execute_after: Option<DateTime<Utc>>,
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(err) => match err {
                WorkerError::UnexpectedError(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                WorkerError::InvalidEmailError(_) => {}
            },
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(connection_pool, email_client).await
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
        execute_after=tracing::field::Empty
    ),
    err
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, WorkerError> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (transaction, task) = task.unwrap();

    Span::current()
        .record("newsletter_issue_id", &display(task.newsletter_issue_id))
        .record("subscriber_email", &display(&task.subscriber_email))
        .record("execute_after", &debug(&task.execute_after));

    match Email::parse(task.subscriber_email.clone()) {
        Ok(email) => {
            let issue = get_issue(pool, task.newsletter_issue_id).await?;
            match email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
                .context("Cannot send email")
            {
                Ok(_) => {
                    delete_task(transaction, &task).await?;
                    Ok(ExecutionOutcome::TaskCompleted)
                }
                Err(err) => {
                    tracing::error!(
                        error.cause_chain = ?err,
                        error.message = %err,
                        "Failed to deliver issue to a confirmed subscriber. \
                        Skipping.",
                    );
                    mark_as_error(transaction, &task).await?;
                    Err(WorkerError::UnexpectedError(err))
                }
            }
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. \
                Their stored contact details are invalid",
            );
            delete_task(transaction, &task).await?;
            Err(WorkerError::InvalidEmailError(task.subscriber_email))
        }
    }
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(pool: &PgPool) -> Result<Option<(PgTransaction, Task)>, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let query = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email, n_retries, execute_after
        FROM issue_delivery_queue
        WHERE COALESCE(execute_after, now()) <= now()
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    );
    let r = transaction.fetch_optional(query).await?;
    if let Some(r) = r {
        let task = Task {
            newsletter_issue_id: r.get("newsletter_issue_id"),
            subscriber_email: r.get("subscriber_email"),
            n_retries: r.get("n_retries"),
            execute_after: r.try_get("execute_after").unwrap_or_default(),
        };
        Ok(Some((transaction, task)))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(mut transaction: PgTransaction, task: &Task) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND
            subscriber_email = $2
        "#,
        task.newsletter_issue_id,
        task.subscriber_email
    );
    transaction.execute(query).await?;
    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
        newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(pool)
    .await?;
    Ok(issue)
}

#[tracing::instrument(skip_all)]
async fn mark_as_error(mut transaction: PgTransaction, task: &Task) -> Result<(), anyhow::Error> {
    let backoff_delay =
        Duration::from_secs(2_u64.pow(task.n_retries as u32).min(MAX_RETRY_DELAY_SEC));
    let execute_after = Utc::now() + backoff_delay;
    transaction
        .execute(sqlx::query!(
            "UPDATE issue_delivery_queue
            SET n_retries = $1, execute_after = $2
            WHERE newsletter_issue_id = $3",
            task.n_retries + 1,
            execute_after,
            task.newsletter_issue_id
        ))
        .await?;
    transaction.commit().await?;
    Ok(())
}
