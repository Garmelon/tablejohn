mod state;
mod r#static;

use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use sqlx::SqlitePool;
use state::AppState;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    number: i32,
}

async fn index(State(db): State<SqlitePool>) -> Result<Response, Response> {
    let result = sqlx::query!("SELECT column1 AS number FROM (VALUES (1))")
        .fetch_one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;

    let number = result.number;
    Ok(IndexTemplate { number }.into_response())
}

async fn run() -> anyhow::Result<()> {
    let state = AppState::new().await?;

    let app = Router::new()
        .route("/", get(index))
        .fallback(get(r#static::static_handler))
        .with_state(state);
    // TODO Add text body to body-less status codes

    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    run().await
}
