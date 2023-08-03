// TODO Occasionally run PRAGMA optimize

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};

// TODO Open db from path
pub async fn pool() -> sqlx::Result<SqlitePool> {
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
        .create_if_missing(true)
        .optimize_on_close(true, None);

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}
