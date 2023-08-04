mod state;
mod r#static;

use std::path::PathBuf;

use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use clap::Parser;
use sqlx::SqlitePool;
use state::AppState;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Debug, clap::Parser)]
#[command(name = NAME, version = VERSION)]
struct Args {
    /// Path to the repo's tablejohn database.
    db: PathBuf,
    /// Path to the git repo.
    repo: PathBuf,
}

fn set_up_logging() {
    let filter = tracing_subscriber::filter::Builder::default()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

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
    // Parse args before any logging starts
    let args = Args::parse();

    set_up_logging();
    info!("You are running {NAME} {VERSION}");

    let state = AppState::new(&args.db).await?;

    let app = Router::new()
        .route("/", get(index))
        .fallback(get(r#static::static_handler))
        .with_state(state);
    // TODO Add text body to body-less status codes
    // TODO Add anyhow-like error type for endpoints

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
