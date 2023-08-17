mod git;
mod recurring;
mod util;
pub mod web;
mod workers;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use axum::extract::FromRef;
use gix::ThreadSafeRepository;
use log::{debug, info};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use tokio::select;

use crate::{args::ServerCommand, config::ServerConfig, somehow};

use self::workers::Workers;

fn open_repo(
    path: &Path,
    url: &Option<String>,
    refspecs: &[String],
) -> somehow::Result<ThreadSafeRepository> {
    if path.exists() {
        info!("Opening repo at {}", path.display());
        Ok(ThreadSafeRepository::open(path)?)
    } else if let Some(url) = url {
        info!(
            "No repo found at {} but a fetch url is configured",
            path.display()
        );

        info!("Creating bare repo");
        git::init_bare(path)?;

        info!("Fetching HEAD from {url}");
        git::fetch_head(path, url)?;

        info!("Fetching refs for the first time (this may take a while)");
        let output = git::fetch(path, url, refspecs)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        info!("Fetched refs:\n{}", stderr.trim_end());

        Ok(ThreadSafeRepository::open(path)?)
    } else {
        Err(somehow::Error(anyhow!(
            "Failed to open repo: No repo found at {} and no fetch url is configured",
            path.display()
        )))
    }
}

fn open_bench_repo(path: &Path) -> somehow::Result<ThreadSafeRepository> {
    if path.exists() {
        info!("Opening bench repo at {}", path.display());
        Ok(ThreadSafeRepository::open(path)?)
    } else {
        Err(somehow::Error(anyhow!(
            "Failed to open bench repo: No repo found at {}",
            path.display()
        )))
    }
}

async fn open_db(path: &Path) -> sqlx::Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        // https://www.sqlite.org/pragma.html#pragma_journal_mode
        .journal_mode(SqliteJournalMode::Wal)
        // https://www.sqlite.org/pragma.html#pragma_synchronous
        // NORMAL recommended when using WAL, can't cause corruption
        .synchronous(SqliteSynchronous::Normal)
        // https://www.sqlite.org/pragma.html#pragma_foreign_keys
        .foreign_keys(true)
        // https://www.sqlite.org/pragma.html#pragma_trusted_schema
        // The docs recommend always turning this off
        .pragma("trusted_schema", "false")
        .filename(path)
        .create_if_missing(true)
        // https://www.sqlite.org/lang_analyze.html#recommended_usage_pattern
        // https://www.sqlite.org/pragma.html#pragma_analysis_limit
        // https://www.sqlite.org/pragma.html#pragma_optimize
        .optimize_on_close(true, Some(1000));

    info!("Opening db at {}", path.display());
    let pool = SqlitePoolOptions::new()
        // Regularly optimize the db as recommended by the sqlite docs
        // https://www.sqlite.org/lang_analyze.html#recommended_usage_pattern
        // https://github.com/launchbadge/sqlx/issues/2111#issuecomment-1254394698
        .max_lifetime(Some(Duration::from_secs(60 * 60 * 24)))
        .connect_with(options)
        .await?;

    debug!("Applying outstanding db migrations");
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

#[derive(Clone)]
pub struct Repo(Arc<ThreadSafeRepository>);

#[derive(Clone)]
pub struct BenchRepo(Arc<ThreadSafeRepository>);

#[derive(Clone, FromRef)]
pub struct Server {
    config: &'static ServerConfig,
    db: SqlitePool,
    repo: Option<Repo>,
    bench_repo: Option<BenchRepo>,
    workers: Arc<Mutex<Workers>>,
}

impl Server {
    pub async fn new(
        config: &'static ServerConfig,
        command: ServerCommand,
    ) -> somehow::Result<Self> {
        let repo = if let Some(path) = command.repo.as_ref() {
            let repo = open_repo(path, &config.repo_fetch_url, &config.repo_fetch_refspecs)?;
            Some(Repo(Arc::new(repo)))
        } else {
            None
        };

        let bench_repo = if let Some(path) = command.bench_repo.as_ref() {
            let repo = open_bench_repo(path)?;
            Some(BenchRepo(Arc::new(repo)))
        } else {
            None
        };

        Ok(Self {
            config,
            db: open_db(&command.db).await?,
            repo,
            bench_repo,
            workers: Arc::new(Mutex::new(Workers::new(config))),
        })
    }

    pub async fn run(&self) -> somehow::Result<()> {
        if let Some(repo) = self.repo.clone() {
            select! {
                e = web::run(self.clone()) => e,
                () = recurring::run(self.clone(), repo) => Ok(()),
            }
        } else {
            web::run(self.clone()).await
        }
    }

    pub async fn shut_down(self) {
        info!("Closing db");
        self.db.close().await;
    }
}
