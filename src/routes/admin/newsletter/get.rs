use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn send_newsletter_issue_form(
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
                    <title>Send a newsletter issue</title>
                </head>
                <body>
                    {msg_html}
                    <form action="/admin/newsletters" method="post">
                        <label>Title:<br>
                            <input
                            type="text"
                            placeholder="Enter title"
                            name="title">
                        </label>
                        <br><br>
                        <label>Plain text content:<br>
                            <textarea
                                placeholder="Enter the content in plain text"
                                name="text_content"
                                rows="15"
                                cols="100"></textarea>
                        </label>
                        <br><br>
                         <label>HTML text content:<br>
                            <textarea
                                placeholder="Enter the content in HTML"
                                name="html_content"
                                rows="15"
                                cols="100"></textarea>
                        </label>
                        <br><br>
                        <button type="submit">Send issue</button>
                    </form>
                    <p><a href="/admin/dashboard">&lt;- Back</a></p>
                </body>
            </html>"#,
        )))
}
