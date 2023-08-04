//! Globally accessible application state.

use std::{path::Path, sync::Arc};

use axum::extract::FromRef;
use gix::ThreadSafeRepository;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use tracing::{debug, info};

use crate::config::Config;

// TODO Occasionally run PRAGMA optimize
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
        .optimize_on_close(true, None);

    info!("Opening db at {}", db_path.display());
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    debug!("Applying outstanding db migrations");
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

fn open_repo(repo_path: &Path) -> anyhow::Result<ThreadSafeRepository> {
    info!("Opening repo at {}", repo_path.display());
    Ok(ThreadSafeRepository::open(repo_path)?)
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: &'static Config,
    pub db: SqlitePool,
    pub repo: Arc<ThreadSafeRepository>,
}

impl AppState {
    pub async fn new(
        config: &'static Config,
        db_path: &Path,
        repo_path: &Path,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            db: open_db(db_path).await?,
            repo: Arc::new(open_repo(repo_path)?),
        })
    }

    pub async fn shut_down(self) {
        info!("Closing db");
        self.db.close().await;
    }
}
