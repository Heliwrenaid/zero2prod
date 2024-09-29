use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use std::time::Duration;

use crate::helpers::{
    assert_is_redirect_to, spawn_app, when_sending_an_email, ConfirmationLinks, TestApp,
};
use wiremock::{matchers::any, Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    app.login_with_admin_user().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act 1 - Publish newsletter
    let newsletter_request_body = create_valid_publish_request_body();
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_form_html().await;

    // Assert
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
    ));
    app.dispatch_all_pending_emails().await.unwrap();
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_admin_user().await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act 1 - Publish newsletter
    let newsletter_request_body = create_valid_publish_request_body();
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_form_html().await;

    // Assert
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
    ));
    app.dispatch_all_pending_emails().await.unwrap();
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];
    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_publish_newsletter(&invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn newsletters_delivery_is_retried_on_transient_error() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_admin_user().await;

    {
        // simulate email service error
        let _guard = when_sending_an_email()
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount_as_scoped(&app.email_server)
            .await;

        // Act 1 - Publish newsletter
        let newsletter_request_body = create_valid_publish_request_body();
        let response = app.post_publish_newsletter(&newsletter_request_body).await;

        // Assert
        assert_is_redirect_to(&response, "/admin/newsletters");

        // Act 2 - Follow the redirect
        let html_page = app.get_publish_newsletter_form_html().await;

        // Assert
        assert!(html_page.contains(
            "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
        ));

        let task = app.fetch_task().await;
        assert_eq!(Some(0), task.n_retries);
        assert!(task.execute_after.is_none());

        // Act 3 - try send email (first time)
        let result = app.dispatch_all_pending_emails().await;

        // Assert
        assert!(result.is_err());

        let task = app.fetch_task().await;
        assert_eq!(Some(1), task.n_retries);
        assert!(task.execute_after.is_some());
    }

    {
        let _guard = when_sending_an_email()
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount_as_scoped(&app.email_server)
            .await;

        // Act 4 - try dispatch email before retry date
        let result = app.dispatch_all_pending_emails().await;
        assert!(result.is_ok());
    }

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act 5 - try dispatch email after retry date
    tokio::time::sleep(Duration::from_secs(3)).await;
    let result = app.dispatch_all_pending_emails().await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn you_must_be_logged_in_to_open_newsletter_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_newsletter_form().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let newsletter_request_body = create_valid_publish_request_body();
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_admin_user().await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act 1 - Submit newsletter form
    let newsletter_request_body = create_valid_publish_request_body();

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_form_html().await;
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
    ));

    // Act 3 - Submit newsletter form **again**
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 4 - Follow the redirect
    let html_page = app.get_publish_newsletter_form_html().await;
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
    ));
    app.dispatch_all_pending_emails().await.unwrap();
    // Mock verifies on Drop that we have sent the newsletter email **once**
}

#[tokio::test]
async fn idempotency_keys_are_removed_after_expiration() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_admin_user().await;

    // Act 1 - Submit newsletter form
    let newsletter_request_body = create_valid_publish_request_body();

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Assert
    let count = app.count_idempotency_keys().await;
    assert_eq!(1, count);

    // Act 2 - try remove idempotency keys
    app.remove_old_idempotency_keys().await;

    // Assert
    let count = app.count_idempotency_keys().await;
    assert_eq!(1, count); // idempotency key is not removed, bcoz it not expired yet

    // Act 3 - create expired idempotency key
    sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, 'some_key', NOW() - INTERVAL '25 hours')
        "#,
        app.admin_user.user_id,
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Assert
    let count = app.count_idempotency_keys().await;
    assert_eq!(2, count);

    // Act 4 - try remove idempotency keys
    app.remove_old_idempotency_keys().await;

    // Assert
    let count = app.count_idempotency_keys().await;
    assert_eq!(1, count);
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.login_with_admin_user().await;
    when_sending_an_email()
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Submit two newsletter forms concurrently
    let newsletter_request_body = create_valid_publish_request_body();
    let response1 = app.post_publish_newsletter(&newsletter_request_body);
    let response2 = app.post_publish_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    app.dispatch_all_pending_emails().await.unwrap();
    // Mock verifies on Drop that we have sent the newsletter email **once**
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    let _mock_guard = when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

fn create_valid_publish_request_body() -> impl serde::Serialize {
    serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    })
}
