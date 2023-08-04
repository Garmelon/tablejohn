//! Globally accessible application state.

use std::{path::Path, sync::Arc, time::Duration};

use axum::extract::FromRef;
use gix::ThreadSafeRepository;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use tracing::{debug, info};

use crate::config::Config;

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

fn open_repo(repo_path: &Path) -> anyhow::Result<ThreadSafeRepository> {
    info!(path = %repo_path.display(), "Opening repo");
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
