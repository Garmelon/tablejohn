// TODO Re-enable and adapt CSS

mod args;
mod config;
mod id;
mod server;
mod shared;
mod somehow;
mod worker;

use std::{
    collections::HashMap,
    io::{self, Write},
    net::IpAddr,
    process,
    time::Duration,
};

use clap::Parser;
use config::ServerConfig;
use log::{debug, error, info, trace, LevelFilter};
use tokio::{select, signal::unix::SignalKind};

use crate::{
    args::{Args, Command, NAME, VERSION},
    config::{Config, WorkerConfig, WorkerServerConfig},
    server::Server,
    worker::Worker,
};

fn set_up_logging(verbose: u8) {
    let level = match verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2.. => LevelFilter::Trace,
    };

    let mut builder = env_logger::builder();
    builder.filter_level(level);
    if verbose <= 2 {
        builder
            .filter_module("hyper", LevelFilter::Warn)
            .filter_module("reqwest", LevelFilter::Warn)
            .filter_module("sqlx", LevelFilter::Warn)
            .filter_module("tracing", LevelFilter::Warn);
    }
    builder
        .format(|f, record| {
            // By prefixing <syslog_level> to the logged messages, they will
            // show up in journalctl with their appropriate level.
            // https://unix.stackexchange.com/a/349148
            // https://0pointer.de/blog/projects/journal-submit.html
            // https://en.wikipedia.org/wiki/Syslog#Severity_level
            let syslog_level = match record.level() {
                log::Level::Error => 3,
                log::Level::Warn => 4,
                log::Level::Info => 6,
                log::Level::Debug | log::Level::Trace => 7,
            };
            let level = {
                let level = record.level();
                let style = f.default_level_style(level);
                format!("{style}{level:>5}{style:#}")
            };
            let args = record.args();
            let module = match record.module_path() {
                Some("tablejohn::server") => Some("server"),
                Some(m) if m.starts_with("tablejohn::server::") => Some("server"),
                Some("tablejohn::worker") => Some("worker"),
                Some(m) if m.starts_with("tablejohn::worker::") => Some("worker"),
                Some("tablejohn") => None,
                Some(m) if m.starts_with("tablejohn") => None,
                Some(m) => Some(m),
                None => None,
            };
            if let Some(module) = module {
                let style = env_logger::fmt::style::Style::new().bold();
                let module = format!("{style}{module}{style:#}");
                writeln!(f, "<{syslog_level}>[{level}] {module}: {args}")
            } else {
                writeln!(f, "<{syslog_level}>[{level}] {args}")
            }
        })
        .init();
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

fn local_url(config: &ServerConfig) -> String {
    let host = match config.web_address.ip() {
        IpAddr::V4(_) => "127.0.0.1",
        IpAddr::V6(_) => "[::1]",
    };
    let port = config.web_address.port();
    let base = &config.web_base;
    format!("http://{host}:{port}{base}")
}

async fn open_in_browser(config: &ServerConfig) {
    // Wait a bit to ensure the server is ready to serve requests.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let url = local_url(config);
    if let Err(e) = open::that_detached(&url) {
        error!("Error opening {url} in browser: {e:?}");
    }
}

async fn launch_local_workers(config: &'static Config, amount: u8) {
    // Wait a bit to ensure the server is ready to serve requests.
    tokio::time::sleep(Duration::from_millis(100)).await;

    for i in 0..amount {
        let mut worker_config = WorkerConfig {
            name: format!("{}-{i:02}", config.worker.name),
            ping: config.worker.ping,
            batch: config.worker.batch,
            servers: HashMap::new(),
        };
        worker_config.servers.insert(
            "localhost".to_string(),
            WorkerServerConfig {
                url: local_url(&config.server),
                token: config.server.worker_token.clone(),
            },
        );
        let worker_config = Box::leak(Box::new(worker_config));

        info!("Starting local worker {}", worker_config.name);
        trace!("Worker config: {worker_config:#?}");
        let worker = Worker::new(worker_config);
        tokio::spawn(async move { worker.run().await });
    }
}

async fn run() -> somehow::Result<()> {
    let args = Args::parse();

    set_up_logging(args.verbose);
    info!("You are running {NAME} {VERSION}");

    let config = Box::leak(Box::new(Config::load(&args)?));

    match args.command {
        Command::Server(command) => {
            info!("Starting server");
            let open = command.open;
            let local_worker = command.local_worker;

            let (server, recurring_rx) = Server::new(&config.server, command).await?;

            if open {
                tokio::task::spawn(open_in_browser(&config.server));
            }

            if local_worker > 0 {
                tokio::task::spawn(launch_local_workers(config, local_worker));
            }

            select! {
                _ = wait_for_signal() => {}
                _ = server.run(recurring_rx) => {}
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
        Command::Worker => {
            info!("Starting worker");

            let worker = Worker::new(&config.worker);

            select! {
                _ = wait_for_signal() => {}
                _ = worker.run() => {}
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    if let Err(e) = run().await {
        error!("{e:?}");
        process::exit(1)
    }
}
