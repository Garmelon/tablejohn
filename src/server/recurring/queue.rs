use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;
use tracing::debug;

use crate::{server::util, somehow};

pub async fn update(db: &SqlitePool) -> somehow::Result<()> {
    debug!("Updating queue");
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    // Get all newly added tracked commits
    let new = sqlx::query!("SELECT hash FROM commits WHERE new AND reachable = 2")
        .fetch_all(&mut *conn)
        .await?;
    let new_len = new.len();

    // Insert them into the queue
    for row in new {
        let id = util::new_run_id();
        let date = OffsetDateTime::now_utc();
        sqlx::query!(
            "INSERT INTO queue (id, hash, date) VALUES (?, ?, ?)",
            id,
            row.hash,
            date
        )
        .execute(&mut *conn)
        .await?;
    }
    debug!("Added {new_len} commits to the queue");

    // Mark all commits as old
    sqlx::query!("UPDATE commits SET new = false")
        .execute(&mut *conn)
        .await?;

    tx.commit().await?;
    debug!("Updated queue");
    Ok(())
}
