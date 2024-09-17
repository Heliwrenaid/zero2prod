use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use secrecy::{ExposeSecret, Secret};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

const TOKEN_EXPIRE_TIMEOUT_IN_DAYS: u8 = 3;

use crate::{
    authentication::{compute_password_hash, UserRole},
    telemetry::spawn_blocking_with_tracing,
    utils::{e500, is_password_invalid, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
    password_check: Secret<String>,
    token: String,
}

impl FormData {
    pub fn validate(&self) -> Result<(), &str> {
        if self.username.is_empty() {
            return Err("Username cannot be empty.");
        }
        if self.password.expose_secret() != self.password_check.expose_secret() {
            return Err("You entered two different passwords - the field values must match.");
        }
        if is_password_invalid(&self.password) {
            return Err(
                "The new password should be longer than 12 characters and shorter than 129 characters",
            );
        }
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TokenCreationTime {
    created_at: DateTime<Utc>,
}

#[tracing::instrument(name = "Confirm collabolator account", skip(form, pool))]
pub async fn activate_account(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let form = form.into_inner();

    if let Err(err_msg) = form.validate() {
        FlashMessage::error(err_msg).send();
        return Ok(redirect_to_form(&form.token));
    }

    if is_user_exists(&form.username, &pool).await.map_err(e500)? {
        FlashMessage::error("User with this username already exists.").send();
        return Ok(redirect_to_form(&form.token));
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")
        .map_err(e500)?;

    let row: Option<TokenCreationTime> = sqlx::query_as!(
        TokenCreationTime,
        "SELECT created_at FROM collabolator_activation_tokens WHERE token = $1 FOR UPDATE",
        &form.token
    )
    .fetch_optional(&mut *transaction)
    .await
    .context("Cannot fetch invitation info from databsae")
    .map_err(e500)?;

    if let Some(token_creation_time) = row {
        if is_token_expired(&token_creation_time) {
            FlashMessage::error("Activation link is expired.").send();
            return Ok(redirect_to_form(&form.token));
        }
        add_new_user_to_db(&form, &mut transaction)
            .await
            .context("Cannot add new user to database")
            .map_err(e500)?;
        delete_activation_token(&form.token, &mut transaction)
            .await
            .context("Cannot delete activation token from database")
            .map_err(e500)?;
        transaction
            .commit()
            .await
            .context("Cannot commit database transaction")
            .map_err(e500)?;
    } else {
        FlashMessage::error("Activation link is invalid.").send();
        return Ok(redirect_to_form(&form.token));
    }

    FlashMessage::info("Account was activated successfully.").send();
    Ok(see_other("/login"))
}

fn redirect_to_form(token: &str) -> HttpResponse {
    let url = format!("/collabolators/activate?token={token}");
    see_other(&url)
}

fn is_token_expired(token_creation_time: &TokenCreationTime) -> bool {
    let now = Utc::now();
    token_creation_time.created_at > now
        || now - token_creation_time.created_at
            > Duration::days(TOKEN_EXPIRE_TIMEOUT_IN_DAYS.into())
}

async fn is_user_exists(username: &str, pool: &PgPool) -> Result<bool, anyhow::Error> {
    let row = sqlx::query!("SELECT user_id from users WHERE username = $1", username)
        .fetch_optional(&*pool)
        .await
        .context("Cannot fetch user from database")?;
    Ok(row.is_some())
}

async fn add_new_user_to_db(
    form: &FormData,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), anyhow::Error> {
    let password = form.password.clone();
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash, role)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::new_v4(),
            form.username,
            password_hash.expose_secret(),
            UserRole::Collabolator.to_string()
        ))
        .await
        .context("Cannot add new user to database")?;
    Ok(())
}

async fn delete_activation_token(
    token: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    transaction
        .execute(sqlx::query!(
            "DELETE FROM collabolator_activation_tokens WHERE token = $1",
            token,
        ))
        .await?;
    Ok(())
}
