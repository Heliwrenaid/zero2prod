use crate::helpers::assert_is_redirect_to;
use crate::helpers::spawn_app;
use crate::helpers::when_sending_an_email;
use reqwest::StatusCode;
use sqlx::PgPool;
use wiremock::ResponseTemplate;

#[tokio::test]
async fn should_invite_new_collabolator() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act 1 - invite new collabolator
    let email = "test_email@abc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/collabolators");
    assert!(token_was_stored_in_db(&app.db_pool, email).await);

    // Act 2 - fetch HTML form
    let html = app.get_invite_form_html().await;

    // Assert
    assert!(html.contains("Activation email was send to new collabolator."));
}

#[tokio::test]
async fn return_error_when_user_is_not_an_admin() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_collabolator_user().await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    let email = "test_email@abc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;

    // Assert
    assert_eq!(StatusCode::FORBIDDEN, response.status());
}

#[tokio::test]
async fn return_error_when_email_is_invalid() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    // Act 1 - invite new collabolator
    let invalid_email = "test_emailabc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": invalid_email
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/collabolators");
    assert!(!token_was_stored_in_db(&app.db_pool, invalid_email).await);

    // Act 2 - fetch HTML form
    let html = app.get_invite_form_html().await;

    // Assert
    assert!(html.contains("Email is invalid."));
}

#[tokio::test]
async fn should_not_send_email_when_token_is_not_saved_in_db() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    // Sabotage the database
    sqlx::query!("ALTER TABLE collabolator_activation_tokens DROP COLUMN token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act 1 - invite new collabolator
    let email = "test_email@abc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;
    // Assert
    assert!(response.status().is_server_error());
}

#[tokio::test]
async fn should_return_error_when_cannot_send_email() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    // Act 1 - invite new collabolator
    let email = "test_email@abc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;
    // Assert
    assert!(response.status().is_server_error());
    assert!(!token_was_stored_in_db(&app.db_pool, email).await);
}

#[tokio::test]
async fn should_resend_activation_link() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    // Act 1 - invite new collabolator
    let email = "test_email@abc.com";
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/collabolators");
    assert!(token_was_stored_in_db(&app.db_pool, email).await);

    // Act 2 - fetch HTML form
    let html = app.get_invite_form_html().await;

    // Assert
    assert!(html.contains("Activation email was send to new collabolator."));

    // Act 3 - resend ivitation
    let response = app
        .post_invite(&serde_json::json!({
            "email": email
        }))
        .await;
    // Assert
    assert_is_redirect_to(&response, "/admin/collabolators");

    // Act 4 - fetch HTML form
    let html = app.get_invite_form_html().await;

    // Assert
    assert!(html.contains("Activation email was send to new collabolator."));
}

async fn token_was_stored_in_db(pool: &PgPool, email: &str) -> bool {
    sqlx::query!(
        "SELECT token FROM collabolator_activation_tokens WHERE email = $1",
        email
    )
    .fetch_optional(pool)
    .await
    .unwrap()
    .is_some()
}
