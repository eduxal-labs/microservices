use crate::services::Authenticator;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use common::types::{Error, Phone};
use serde::{Deserialize, Serialize};

pub fn router(authenticator: Authenticator) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/verify", post(verify))
        .route("/setup", post(setup))
        .route("/refresh", get(refresh).post(refresh))
        .with_state(authenticator)
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    phone: Phone,
}

#[derive(Debug, Deserialize)]
struct VerifyRequest {
    phone: Phone,
    code: String,
}

#[derive(Debug, Serialize)]
struct VerifyResponse {
    token: String,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct SetupRequest {
    name: String,
    device: String,
}

#[axum::debug_handler]
async fn login(
    State(authenticator): State<Authenticator>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, AuthError> {
    let verification = authenticator.login(payload.phone).await?;
    Ok((StatusCode::OK, Json(verification)).into_response())
}

#[axum::debug_handler]
async fn verify(
    State(authenticator): State<Authenticator>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Response, AuthError> {
    let (setup_token, upload_url) = authenticator.verify(payload.phone, &payload.code).await?;
    let token_str = setup_token.tokenize().map_err(|_| Error::InvalidToken)?;

    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            token: token_str,
            upload_url,
        }),
    )
        .into_response())
}

#[axum::debug_handler]
async fn setup(
    State(authenticator): State<Authenticator>,
    headers: HeaderMap,
    Json(payload): Json<SetupRequest>,
) -> Result<Response, AuthError> {
    let token_str = extract_bearer_token(&headers)?;
    let bundle = authenticator
        .setup(&token_str, payload.name, payload.device)
        .await?;

    Ok((StatusCode::OK, Json(bundle)).into_response())
}

#[axum::debug_handler]
async fn refresh(
    State(authenticator): State<Authenticator>,
    headers: HeaderMap,
) -> Result<Response, AuthError> {
    let token_str = extract_bearer_token(&headers)?;
    let bundle = authenticator.refresh(&token_str).await?;

    Ok((StatusCode::OK, Json(bundle)).into_response())
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, AuthError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError(Error::InvalidToken))?;

    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        Ok(token.trim().to_string())
    } else {
        Err(AuthError(Error::InvalidToken))
    }
}

pub struct AuthError(Error);

impl From<Error> for AuthError {
    fn from(err: Error) -> Self {
        AuthError(err)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            Error::SlowDown => StatusCode::TOO_MANY_REQUESTS,
            Error::InvalidToken => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(serde_json::json!({
            "error": format!("{:?}", self.0)
        }));

        (status, body).into_response()
    }
}
