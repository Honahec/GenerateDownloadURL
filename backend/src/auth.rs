use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    #[allow(dead_code)]
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    InvalidFormat,
    MissingState,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::MissingState => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::UNAUTHORIZED,
        };
        let message = match self {
            AuthError::MissingToken => "Authorization header is missing",
            AuthError::InvalidToken => "Authorization token is invalid",
            AuthError::InvalidFormat => {
                "Authorization header must be in the format 'Bearer <token>'"
            }
            AuthError::MissingState => "Application state is unavailable",
        };
        (status, message).into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::State;

        println!("Auth check for path: {}", parts.uri.path());

        let State(app_state) = State::<AppState>::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::MissingState)?;

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                println!("Missing authorization header for path: {}", parts.uri.path());
                AuthError::MissingToken
            })?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                println!("Invalid auth format for path: {}", parts.uri.path());
                AuthError::InvalidFormat
            })?;

        let decoded = decode::<Claims>(
            token,
            &DecodingKey::from_secret(app_state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            println!("Invalid token for path {}: {}", parts.uri.path(), e);
            AuthError::InvalidToken
        })?;

        println!("Auth successful for user: {} on path: {}", decoded.claims.sub, parts.uri.path());
        Ok(Self {
            username: decoded.claims.sub,
        })
    }
}

pub fn generate_token(
    username: &str,
    config: &AppConfig,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize;
    let exp = now + (config.jwt_exp_minutes as usize * 60);

    let claims = Claims {
        sub: username.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}
