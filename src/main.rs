mod config;
mod recurring;
mod repo;
mod somehow;
mod state;
mod web;

use std::{io, path::PathBuf};

use clap::Parser;
use directories::ProjectDirs;
use state::AppState;
use tokio::{select, signal::unix::SignalKind};
use tracing::{debug, info, Level};
use tracing_subscriber::{
    filter::Targets, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

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
    /// Enable increasingly more verbose output
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn set_up_logging(verbose: u8) {
    let filter = Targets::new()
        .with_default(Level::TRACE)
        .with_target("hyper", Level::INFO)
        .with_target("sqlx", Level::INFO);
    match verbose {
        0 => tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .without_time()
            .with_target(false)
            .init(),
        1 => tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .with_target(false)
            .finish()
            .with(filter)
            .init(),
        2 => tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .pretty()
            .finish()
            .with(filter)
            .init(),
        _ => tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .pretty()
            .init(),
    }
}

fn load_config(path: Option<PathBuf>) -> somehow::Result<&'static Config> {
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

async fn run() -> somehow::Result<()> {
    let args = Args::parse();

    set_up_logging(args.verbose);
    info!("You are running {NAME} {VERSION}");

    let config = load_config(args.config)?;
    let state = AppState::new(config, &args.db, &args.repo).await?;

    info!("Startup complete, running");
    select! {
        _ = wait_for_signal() => {},
        _ = web::run(state.clone()) => {},
        _ = recurring::run(state.clone()) => {},
    }

    state.shut_down().await;

    Ok(())
}

#[tokio::main]
async fn main() -> somehow::Result<()> {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    run().await
}
