use reqwest::StatusCode;

use crate::helpers::spawn_app;

#[tokio::test]
async fn return_error_when_query_is_missing() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_account_activate_form(&()).await;

    // Assert
    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn should_fetch_html_when_token_is_set_in_query() {
    // Arrange
    let app = spawn_app().await;
    let token = "1239754769274fdgfknbser";

    // Act
    let html = app.get_account_activate_form_html(&token).await;

    // Assert
    assert!(html.contains(r#"<form action="/collabolators/activate" method="post">"#));
    assert!(html.contains(&format!(
        r#"<input type="hidden" name="token" value="{token}">"#
    )))
}
