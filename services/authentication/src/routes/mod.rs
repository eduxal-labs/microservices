use crate::services::Authenticator;
use axum::Router;

pub mod authentication;

pub fn router(authenticator: Authenticator) -> Router {
    authentication::router(authenticator)
}
