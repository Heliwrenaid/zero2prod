use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::{ErrorForbidden, InternalError};
use actix_web::FromRequest;
use actix_web::HttpMessage;
use actix_web_lab::middleware::Next;
use std::fmt::Debug as _;
use std::ops::Deref;

use super::{UserData, UserRole};

const PERMISSION_DENIED_ERR_MSG: &str = "User has no permission to access this endpoint";

#[derive(Clone, Debug)]
pub struct AuthenticatedUser(UserData);

impl std::fmt::Display for AuthenticatedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for AuthenticatedUser {
    type Target = UserData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user().map_err(e500)? {
        Some(user) => {
            req.extensions_mut().insert(AuthenticatedUser(user));
            next.call(req).await
        }
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}

pub async fn reject_not_admin_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user().map_err(e500)? {
        Some(user) => match user.role {
            UserRole::Admin => {
                req.extensions_mut().insert(AuthenticatedUser(user));
                next.call(req).await
            }
            _ => {
                let e = anyhow::anyhow!(PERMISSION_DENIED_ERR_MSG);
                Err(ErrorForbidden(e))
            }
        },
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}
