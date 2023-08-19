use log::{debug, info, warn};
use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;

use crate::somehow;

async fn inner(db: &SqlitePool) -> somehow::Result<()> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    // Get all newly added tracked commits
    let new = sqlx::query!("SELECT hash FROM commits WHERE new AND reachable = 2")
        .fetch_all(&mut *conn)
        .await?;
    debug!("Found {} new commits", new.len());

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

    // Mark all commits we just added to the queue as old. Commits from
    // untracked branches are not marked as old because otherwise we'd miss them
    // when they eventually end up in the tracked branches. This can for example
    // happen when a tracked branch is fast-forwarded to a commit from an
    // untracked branch.
    //
    // When tracked refs are updated, all new commits are automatically added to
    // the queue, since they were still new and have now transitioned to
    // reachable = 2. This should hopefully not be too big of a problem since
    // usually the main branch is also tracked. I think I'd rather implement
    // better queue management tools and graph UI than change this behaviour.
    let amount = sqlx::query!("UPDATE commits SET new = false WHERE reachable = 2")
        .execute(&mut *conn)
        .await?
        .rows_affected();
    debug!("Marked {amount} commits as old");

    tx.commit().await?;
    Ok(())
}

pub(super) async fn update(db: &SqlitePool) {
    debug!("Updating queue");
    if let Err(e) = inner(db).await {
        warn!("Error updating queue:\n{e:?}");
    }
}
