mod args;
mod config;
mod id;
mod runner;
mod server;
mod shared;
mod somehow;

use std::{io, process, time::Duration};

use clap::Parser;
use tokio::{select, signal::unix::SignalKind};
use tracing::{debug, error, info, Level};
use tracing_subscriber::{
    filter::Targets, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    args::{Args, Command, NAME, VERSION},
    config::Config,
    runner::Runner,
    server::Server,
};

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

async fn wait_for_signal() -> io::Result<()> {
    debug!("Listening to signals");

    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

    let signal = select! {
        _ = sigint.recv() => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    };

    info!("Received signal ({signal}), shutting down gracefully");
    Ok(())
}

async fn die_on_signal() -> io::Result<()> {
    debug!("Listening to signals again");

    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;

    let signal = select! {
        _ = sigint.recv() => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    };

    error!("Received second signal ({signal}), dying immediately");
    process::exit(1);
}

async fn open_in_browser(config: &Config) {
    // Wait a bit to ensure the server is ready to serve requests.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let url = format!("http://{}{}", config.web_address, config.web_base);
    if let Err(e) = open::that_detached(&url) {
        error!("Error opening {url} in browser: {e:?}");
    }
}

async fn run() -> somehow::Result<()> {
    let args = Args::parse();

    set_up_logging(args.verbose);
    info!("You are running {NAME} {VERSION}");

    let config = Box::leak(Box::new(Config::load(&args)?));

    match args.command {
        Command::Server(command) => {
            if command.open {
                tokio::task::spawn(open_in_browser(config));
            }

            let server = Server::new(config, command).await?;

            select! {
                _ = wait_for_signal() => {}
                _ = server.run() => {}
            }

            select! {
                _ = die_on_signal() => {}
                // For some reason, the thread pool shutting down seems to block
                // receiving further signals if a heavy sql operation is currently
                // running. Maybe this is due to the thread pool not deferring blocking
                // work to a separate thread? In any case, replacing it with a sleep
                // doesn't block the signals.
                //
                // In order to fix this, I could maybe register a bare signal handler
                // (instead of using tokio streams) that just calls process::exit(1) and
                // nothing else?
                _ = server.shut_down() => {}
            }
        }
        Command::Runner => {
            let runner = Runner::new(config);

            select! {
                _ = wait_for_signal() => {}
                _ = runner.run() => {}
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> somehow::Result<()> {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    run().await
}
