use log::{debug, info, warn};
use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;

use crate::somehow;

async fn inner(db: &SqlitePool) -> somehow::Result<()> {
    debug!("Updating queue");
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    // Get all newly added tracked commits
    let new = sqlx::query!("SELECT hash FROM commits WHERE new AND reachable = 2")
        .fetch_all(&mut *conn)
        .await?;

    // Insert them into the queue
    for row in new {
        let date = OffsetDateTime::now_utc();
        let result = sqlx::query!(
            "INSERT OR IGNORE INTO queue (hash, date) VALUES (?, ?)",
            row.hash,
            date,
        )
        .execute(&mut *conn)
        .await?;

        if result.rows_affected() > 0 {
            info!("Added new commit {} to the queue", row.hash);
        }
    }

    // Mark all commits as old
    sqlx::query!("UPDATE commits SET new = false")
        .execute(&mut *conn)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub(super) async fn update(db: &SqlitePool) {
    if let Err(e) = inner(db).await {
        warn!("Error updating queue:\n{e:?}");
    }
}
