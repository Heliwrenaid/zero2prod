use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use chrono::Utc;
use sqlx::{Executor, PgPool, Postgres, Transaction};

use crate::{
    domain::Email,
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
    utils::{e500, generate_token, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
}

#[tracing::instrument(
    name = "Invite a new collaborator",
    skip(form, email_client, base_url, pool)
)]
pub async fn invite_collaborator(
    form: web::Form<FormData>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let email = Email::parse(form.into_inner().email);
    if email.is_err() {
        FlashMessage::error("Email is invalid.").send();
        return Ok(redirect_to_form());
    }
    let email = email.unwrap();
    let token = generate_token();

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")
        .map_err(e500)?;

    store_activation_token(&mut transaction, &email, &token)
        .await
        .map_err(e500)?;

    send_activation_email(&email_client, &email, &base_url.get_ref().0, &token)
        .await
        .map_err(e500)?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")
        .map_err(e500)?;

    FlashMessage::info("Activation email was send to new collabolator.").send();
    Ok(redirect_to_form())
}

fn redirect_to_form() -> HttpResponse {
    see_other("/admin/collabolators")
}

#[tracing::instrument(
    name = "Store an activation token",
    skip(transaction, email, activation_token)
)]
async fn store_activation_token(
    transaction: &mut Transaction<'_, Postgres>,
    email: &Email,
    activation_token: &str,
) -> Result<(), sqlx::Error> {
    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO collabolator_activation_tokens (email, token, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (email) 
            DO UPDATE SET token = EXCLUDED.token, created_at = EXCLUDED.created_at;
            "#,
            email.as_ref(),
            activation_token,
            Utc::now()
        ))
        .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a activation email to a new collaborator",
    skip(email_client, email, base_url, activation_token)
)]
async fn send_activation_email(
    email_client: &EmailClient,
    email: &Email,
    base_url: &str,
    activation_token: &str,
) -> Result<(), reqwest::Error> {
    let activation_link = format!(
        "{}/collabolators/activate?token={}",
        base_url, activation_token
    );
    let plain_body = format!(
        "Do you want to be a collabolator of our newsletter?\nVisit {} to activate your collaborator account.",
        activation_link
    );
    let html_body = format!(
        "Do you want to be a collabolator of our newsletter?<br />\
        Click <a href=\"{}\">here</a> to activate your collaborator account.",
        activation_link
    );
    email_client
        .send_email(email, "Account activation", &html_body, &plain_body)
        .await
}
