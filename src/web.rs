mod index;
mod r#static;

use std::{error, result};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router, Server,
};

use crate::state::AppState;

/// Anyhow-like error that also implements [`IntoResponse`].
pub struct Error(anyhow::Error);

impl<E> From<E> for Error
where
    E: error::Error + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self(anyhow::Error::from(value))
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

/// Anyhow-like result that also implements [`IntoResponse`].
pub type Result<T> = result::Result<T, Error>;

pub async fn run(state: AppState) -> anyhow::Result<()> {
    // TODO Add text body to body-less status codes

    let app = Router::new()
        .route("/", get(index::get))
        .fallback(get(r#static::static_handler))
        .with_state(state.clone());

    Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
