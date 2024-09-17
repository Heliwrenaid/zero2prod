use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write as _;

pub async fn invite_collaborator_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Invite Collabolator</title>
                </head>
                <body>
                    {msg_html}
                    <form action="/admin/collabolators" method="post">
                        <label>Collabolator email
                        <input
                        type="text"
                        placeholder="Enter email"
                        name="email"
                        >
                        <button type="submit">Invite</button>
                    </form>
                    <p><a href="/admin/dashboard">&lt;- Back</a></p>
                </body>
            </html>"#,
        )))
}
