mod config;
mod state;
mod r#static;
mod update;

use std::{io, path::PathBuf};

use askama::Template;
use askama_axum::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, routing::get, Router};
use clap::Parser;
use directories::ProjectDirs;
use sqlx::SqlitePool;
use state::AppState;
use tokio::{select, signal::unix::SignalKind};
use tracing::{debug, info};
use tracing_subscriber::filter::LevelFilter;

use crate::config::Config;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Debug, clap::Parser)]
#[command(name = NAME, version = VERSION)]
struct Args {
    /// Path to the repo's tablejohn database.
    db: PathBuf,
    /// Path to the git repo.
    repo: PathBuf,
    /// Path to the config file.
    #[arg(long, short)]
    config: Option<PathBuf>,
    /// Enable more verbose output
    #[arg(long, short)]
    verbose: bool,
}

fn set_up_logging(verbose: bool) {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::TRACE)
            .pretty()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::INFO)
            .without_time()
            .with_target(false)
            .init();
    }
}

fn load_config(path: Option<PathBuf>) -> anyhow::Result<&'static Config> {
    let config_path = path.unwrap_or_else(|| {
        ProjectDirs::from("de", "plugh", "tablejohn")
            .expect("could not determine home directory")
            .config_dir()
            .join("config.toml")
    });

    Ok(Box::leak(Box::new(Config::load(&config_path)?)))
}

async fn wait_for_signal() -> io::Result<()> {
    debug!("Listening to signals");

    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

    let signal = select! {
        _ = sigint.recv() => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    };

    info!("Received {signal}, shutting down");
    Ok(())
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
    let args = Args::parse();

    set_up_logging(args.verbose);
    info!("You are running {NAME} {VERSION}");

    let config = load_config(args.config)?;
    let state = AppState::new(config, &args.db, &args.repo).await?;

    let app = Router::new()
        .route("/", get(index))
        .fallback(get(r#static::static_handler))
        .with_state(state.clone())
        .into_make_service();
    // TODO Add text body to body-less status codes
    // TODO Add anyhow-like error type for endpoints

    let server = axum::Server::bind(&"0.0.0.0:8000".parse().unwrap());

    info!("Startup complete, running");
    select! {
        _ = wait_for_signal() => {},
        _ = server.serve(app) => {},
        _ = update::repeatedly(state.clone()) => {},
    }

    state.shut_down().await;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    run().await
}
