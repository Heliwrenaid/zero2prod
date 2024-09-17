use actix_session::SessionExt;
use actix_session::{Session, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{ready, Ready};

use crate::authentication::UserData;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_KEY: &'static str = "user";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user(&self, user: UserData) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_KEY, user)
    }

    pub fn get_user(&self) -> Result<Option<UserData>, SessionGetError> {
        self.0.get(Self::USER_KEY)
    }

    pub fn log_out(self) {
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;

    type Future = Ready<Result<TypedSession, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
