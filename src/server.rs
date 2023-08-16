mod recurring;
mod util;
pub mod web;
mod workers;

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::extract::FromRef;
use gix::ThreadSafeRepository;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use tokio::select;
use tracing::{debug, info};

use crate::{args::ServerCommand, config::ServerConfig, somehow};

use self::workers::Workers;

async fn open_db(db_path: &Path) -> sqlx::Result<SqlitePool> {
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
        .filename(db_path)
        .create_if_missing(true)
        // https://www.sqlite.org/lang_analyze.html#recommended_usage_pattern
        // https://www.sqlite.org/pragma.html#pragma_analysis_limit
        // https://www.sqlite.org/pragma.html#pragma_optimize
        .optimize_on_close(true, Some(1000));

    info!(path = %db_path.display(), "Opening db");
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
            info!(path = %path.display(), "Opening repo");
            let repo = ThreadSafeRepository::open(path)?;
            Some(Repo(Arc::new(repo)))
        } else {
            None
        };

        let bench_repo = if let Some(path) = command.bench_repo.as_ref() {
            info!(path = %path.display(), "Opening repo");
            let repo = ThreadSafeRepository::open(path)?;
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
