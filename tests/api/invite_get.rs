use reqwest::StatusCode;

use crate::helpers::spawn_app;

#[tokio::test]
async fn return_error_when_user_is_not_an_admin() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_collabolator_user().await;

    // Act
    let response = app.get_invite_form().await;

    // Assert
    assert_eq!(StatusCode::FORBIDDEN, response.status());
}

#[tokio::test]
async fn should_fetch_html_form_for_authorized_user() {
    // Arrange
    let app = spawn_app().await;
    app.login_with_admin_user().await;

    // Act
    let response = app.get_invite_form().await;

    // Assert
    assert!(response.status().is_success());
    let html = response.text().await.unwrap();
    assert!(html.contains(r#"<form action="/admin/collabolators" method="post"#))
}
