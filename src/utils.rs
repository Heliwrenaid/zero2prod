use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use rand::distributions::Alphanumeric;
use rand::Rng;
use secrecy::{ExposeSecret, Secret};

// Return an opaque 500 while preserving the error root's cause for logging.
pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

// Return a 400 with the user-representation of the validation error as body.
// The error root cause is preserved for logging purposes.
pub fn e400<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

/// Generate a random 25-characters-long case-sensitive token.
pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

pub fn is_password_invalid(password: &Secret<String>) -> bool {
    let new_pass_len = password.expose_secret().len();
    new_pass_len < 13 || new_pass_len > 128
}
