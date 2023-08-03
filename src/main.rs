mod db;
mod r#static;

use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::{http::StatusCode, routing::get, Extension, Router};
use sqlx::{Row, SqlitePool};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    number: i32,
}

async fn index(Extension(pool): Extension<SqlitePool>) -> Result<Response, Response> {
    let result = sqlx::query("SELECT * FROM (VALUES (1))")
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;

    let number: i32 = result.get(0);
    Ok(IndexTemplate { number }.into_response())
}

async fn run() -> anyhow::Result<()> {
    let pool = db::pool().await?;

    let app = Router::new()
        .route("/", get(index))
        .fallback(get(r#static::static_handler))
        .layer(Extension(pool));
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
