use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

#[derive(serde::Deserialize)]
pub struct QueryData {
    token: String,
}

pub async fn activate_account_form(
    query: web::Query<QueryData>,
    flash_messages: IncomingFlashMessages,
) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    let token = query.into_inner().token;
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Activate account</title>
                </head>
                <body>
                    {error_html}
                        <form action="/collabolators/activate" method="post">
                            <label>Username
                            <input type="text" placeholder="Enter Username" name="username">
                            </label>
                            <label>Password
                            <input type="password" placeholder="Enter password" name="password">
                            <label>Confirm password
                            <input type="password" placeholder="Enter Password" name="password_check">
                            </label>
                            <input type="hidden" name="token" value="{token}">
                            <button type="submit">Activate account</button>
                        </form>
                    </body>
            </html>"#,
        ))
}
