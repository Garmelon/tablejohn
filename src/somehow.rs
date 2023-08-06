use std::{error, fmt, result};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Wrapper around [`anyhow::Error`] that implements additional type classes.
pub struct Error(pub anyhow::Error);

impl Error {
    pub fn from_box(err: Box<dyn error::Error + Send + Sync + 'static>) -> Self {
        Self(anyhow::anyhow!(err))
    }
}

impl<E> From<E> for Error
where
    E: error::Error + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self(anyhow::Error::from(value))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("500 Internal Server Error\n\n{}", self.0),
        )
            .into_response()
    }
}

pub type Result<T> = result::Result<T, Error>;
