//! Verify worker basic authentication headers.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};

use crate::config::ServerConfig;

fn is_username_valid(username: &str) -> bool {
    if username.is_empty() {
        return false;
    }

    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn is_password_valid(password: &str, config: &'static ServerConfig) -> bool {
    password == config.worker_token
}

pub fn authenticate(
    config: &'static ServerConfig,
    auth: Option<TypedHeader<Authorization<Basic>>>,
) -> Result<String, Response> {
    if let Some(auth) = auth {
        if is_username_valid(auth.username()) && is_password_valid(auth.password(), config) {
            return Ok(auth.username().to_string());
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Basic realm=\"worker api\""),
        )],
        "invalid credentials",
    )
        .into_response())
}
