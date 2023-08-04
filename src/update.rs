//! Repeatedly update the db from the repo.

use std::collections::HashSet;

use anyhow::anyhow;
use futures::TryStreamExt;
use gix::{ObjectId, Repository};
use sqlx::{prelude::*, SqliteConnection, SqlitePool};
use tracing::{debug, debug_span, error, Instrument};

use crate::state::AppState;

/// Add new commits from the repo to the database, marked as new.
///
/// Starts at the known refs and advances depth-first until it hits a commit
/// that is already in the db.
///
/// Uses a transaction because batch inserts in sqlite are a lot faster in
/// transactions.
// TODO Initialize tracked refs?
// TODO Update tracked refs?
async fn add_new_commits_to_db(db: &SqlitePool, repo: &Repository) -> anyhow::Result<()> {
    debug!("Adding new commits to the db");
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    // Defer foreign key checks until the end of the transaction to improve
    // insert performance.
    sqlx::query!("PRAGMA defer_foreign_keys=1")
        .execute(&mut *conn)
        .await?;

    let commits = get_all_commits_from_db(&mut *conn).await?;
    debug!("Loaded {} commits from the db", commits.len());

    let mut references = vec![];
    for reference in repo.references()?.prefixed("refs")? {
        let id: ObjectId = reference
            .map_err(|e| anyhow!(e))?
            .into_fully_peeled_id()?
            .into();
        references.push(id);
    }
    debug!("Found {} refs in repo", references.len());

    let new_commits = repo
        .rev_walk(references)
        .selected(|c| !commits.contains(c))?
        .collect::<Result<Vec<_>, _>>()?;
    debug!("Found {} new commits in repo", new_commits.len());

    for commit in new_commits {
        let hash = commit.id.to_string();
        sqlx::query!("INSERT OR IGNORE INTO commits (hash) VALUES (?)", hash)
            .execute(&mut *conn)
            .await?;

        for parent in commit.parent_ids() {
            let parent_hash = parent.to_string();
            sqlx::query!(
                "INSERT INTO commit_links (parent, child) VALUES (?, ?)",
                parent_hash,
                hash
            )
            .execute(&mut *conn)
            .await?;
        }
    }

    debug!("Finished adding new commits to the db");
    tx.commit().await?;
    Ok(())
}

async fn get_all_commits_from_db(conn: &mut SqliteConnection) -> anyhow::Result<HashSet<ObjectId>> {
    let hashes = sqlx::query!("SELECT hash FROM commits")
        .fetch(conn)
        .err_into::<anyhow::Error>()
        .and_then(|r| async move { r.hash.parse::<ObjectId>().map_err(|e| e.into()) })
        .try_collect::<HashSet<_>>()
        .await?;

    Ok(hashes)
}

async fn update_repo(state: &AppState) -> anyhow::Result<()> {
    let repo = state.repo.to_thread_local();

    add_new_commits_to_db(&state.db, &repo)
        .instrument(debug_span!("add new commits"))
        .await?;

    Ok(())
}

pub async fn repeatedly(state: AppState) {
    loop {
        async {
            if let Err(e) = update_repo(&state).await {
                error!("{e:?}");
            }
        }
        .instrument(debug_span!("update repo"))
        .await;

        tokio::time::sleep(state.config.repo_update_delay).await;
    }
}
