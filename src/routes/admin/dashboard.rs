use crate::authentication::{AuthenticatedUser, UserRole};
use actix_web::{http::header::ContentType, web, HttpResponse};

pub async fn admin_dashboard(
    user: web::ReqData<AuthenticatedUser>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = user.username.as_str();
    let invite_link = match user.role {
        UserRole::Admin => {
            r#"<li><a href="/admin/collabolators">Invite a new collaborator</a></li>"#
        }
        _ => "",
    };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Admin dashboard</title>
                </head>
                <body>
                    <p>Welcome {username}!</p>
                    <p>Available actions:</p>
                    <ol>
                        <li><a href="/admin/password">Change password</a></li>
                        <li><a href="/admin/newsletters">Send a newsletter issue</a></li>
                        {invite_link}
                        <li>
                            <form name="logoutForm" action="/admin/logout" method="post">
                                <input type="submit" value="Logout">
                            </form>
                        </li>
                    </ol>
                </body>
            </html>"#
        )))
}
