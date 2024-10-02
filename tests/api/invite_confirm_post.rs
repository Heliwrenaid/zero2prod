use chrono::{Duration, Utc};
use reqwest::Response;
use sqlx::PgPool;
use zero2prod::authentication::{User, UserRole};

use crate::helpers::{assert_is_redirect_to, spawn_app};

const DEFAULT_TOKEN: &str = "kwwa2wjfgjk34673jdfg";

#[tokio::test]
async fn return_error_when_form_data_is_invalid() {
    // Arrange
    let app = spawn_app().await;

    let pass_129 = generate_pass_of_size(129);
    let username = "user234";
    let scenarios = vec![
        (
            generate_form_data(username, "12345678901234", "77775678901237"),
            "You entered two different passwords - the field values must match.",
        ),
        (
            generate_form_data(username, "12345678901", "12345678901"),
            "The new password should be longer than 12 characters and shorter than 129 characters",
        ),
        (
            generate_form_data(username, "123456789012", "123456789012"),
            "The new password should be longer than 12 characters and shorter than 129 characters",
        ),
        (
            generate_form_data(username, "123456789012", "123456789012"),
            "The new password should be longer than 12 characters and shorter than 129 characters",
        ),
        (
            generate_form_data(username, &pass_129, &pass_129),
            "The new password should be longer than 12 characters and shorter than 129 characters",
        ),
        (
            generate_form_data("", "12345678901234", "77775678901237"),
            "Username cannot be empty.",
        ),
        (
            generate_form_data(&app.admin_user.username, "12345678901234", "12345678901234"),
            "User with this username already exists.",
        ),
    ];

    // Act
    for scenario in scenarios {
        let response = app.post_account_activate(&scenario.0).await;
        assert_is_redirect_to_form(&response);

        let html = app.get_account_activate_form_html(DEFAULT_TOKEN).await;
        assert!(html.contains(scenario.1));
    }
}

#[tokio::test]
async fn return_error_when_token_not_exists_in_db() {
    // Arrange
    let app = spawn_app().await;
    let pass = generate_pass_of_size(20);
    let username = "user123456";

    // Act 1 - activate account
    let form = generate_form_data(username, &pass, &pass);
    let response = app.post_account_activate(&form).await;

    // Assert
    assert_is_redirect_to_form(&response);

    // Act 2 - follow redirect
    let html = app.get_account_activate_form_html(DEFAULT_TOKEN).await;
    assert!(html.contains("Activation link is invalid."));
}

#[tokio::test]
async fn return_error_when_token_is_expired() {
    // Arrange
    let app = spawn_app().await;
    let pass = generate_pass_of_size(20);
    let username = "user123456";

    sqlx::query!(
        r#"
        INSERT INTO collabolator_activation_tokens (email, token, created_at)
        VALUES ($1, $2, $3)
        "#,
        "test_email@o.com",
        DEFAULT_TOKEN,
        Utc::now() - Duration::days(4) // too old timestamp
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Act 1 - activate account
    let form = generate_form_data(username, &pass, &pass);
    let response = app.post_account_activate(&form).await;

    // Assert
    assert_is_redirect_to_form(&response);

    // Act 2 - follow redirect
    let html = app.get_account_activate_form_html(DEFAULT_TOKEN).await;
    assert!(html.contains("Activation link is expired."));
}

#[tokio::test]
async fn should_activate_new_account() {
    // Arrange
    let app = spawn_app().await;
    let email = "test_email@a.com";
    let pass = generate_pass_of_size(20);
    let username = "user123456";

    sqlx::query!(
        r#"
        INSERT INTO collabolator_activation_tokens (email, token, created_at)
        VALUES ($1, $2, $3)
        "#,
        email,
        DEFAULT_TOKEN,
        Utc::now() - Duration::days(1)
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Act 1 - activate account
    let form = generate_form_data(username, &pass, &pass);
    let response = app.post_account_activate(&form).await;

    // Assert
    assert_is_redirect_to(&response, "/login");
    let user = get_user_from_db(username, &app.db_pool).await;
    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(username, user.username);
    assert!(matches!(user.role, UserRole::Collabolator));

    // Act 2 - follow redirect
    let html = app.get_login_html().await;

    // Assert
    assert!(html.contains("Account was activated successfully."));

    // Act 3 - try to login with new user (in order to test password)
    let response = app
        .post_login(&serde_json::json!({
            "username": username,
            "password": pass
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/dashboard");
}

fn generate_form_data(
    username: &str,
    password: &str,
    password_check: &str,
) -> impl serde::Serialize {
    serde_json::json!({
        "username": username,
        "password": password,
        "password_check": password_check,
        "token": DEFAULT_TOKEN,
    })
}

fn generate_pass_of_size(size: usize) -> String {
    let mut s = String::new();
    for _ in 0..size {
        s.push('0');
    }
    s
}

fn assert_is_redirect_to_form(response: &Response) {
    let expected_redirection_url = format!("/collabolators/activate?token={DEFAULT_TOKEN}");
    assert_is_redirect_to(response, &expected_redirection_url);
}

async fn get_user_from_db(username: &str, pool: &PgPool) -> Option<User> {
    sqlx::query_as!(
        User,
        r#"
        SELECT user_id, username, password_hash, role as "role: UserRole"
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}
