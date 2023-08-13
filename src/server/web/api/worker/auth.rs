//! Verify worker basic authentication headers.

use axum::{
    headers::{authorization::Basic, Authorization},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    TypedHeader,
};

use crate::config::Config;

fn is_username_valid(username: &str) -> bool {
    if username.is_empty() {
        return false;
    }

    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn is_password_valid(password: &str, config: &'static Config) -> bool {
    password == config.web_worker_token
}

pub fn authenticate(
    config: &'static Config,
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
