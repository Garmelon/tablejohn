//! Repeatedly update the db from the repo.

use std::collections::HashSet;

use anyhow::anyhow;
use futures::TryStreamExt;
use gix::{objs::Kind, traverse::commit::Info, ObjectId, Repository};
use sqlx::{prelude::*, SqliteConnection, SqlitePool};
use tracing::{debug, debug_span, error, info, Instrument};

use crate::state::AppState;

/// Add new commits from the repo to the database, marked as new.
// TODO Rename so the name fits better (maybe rename update module to recurring as well?)
// upadte module -> recurring
// Fetching new commits -> fetch submodule
// Updating repo (current file) -> repo submodule
// Updating queue -> queue submodule
async fn add_new_commits_to_db(db: &SqlitePool, repo: &Repository) -> anyhow::Result<()> {
    debug!("Adding new commits to db");
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let old = get_all_commits_from_db(&mut *conn).await?;
    debug!("Loaded {} commits from the db", old.len());

    let new = get_new_commits_from_repo(repo, &old)?;
    debug!("Found {} new commits in repo", new.len());

    // Defer foreign key checks until the end of the transaction to improve
    // insert performance.
    sqlx::query!("PRAGMA defer_foreign_keys=1")
        .execute(&mut *conn)
        .await?;

    // Inserts are grouped by table so sqlite can process them *a lot* faster
    // than if they were grouped by commit (insert commit and parents, then next
    // commit and so on).
    insert_new_commits(conn, &new).await?;
    insert_new_commit_links(conn, &new).await?;
    debug!("Inserted {} new commits into db", new.len());

    if old.is_empty() {
        // We've never seen any repo before, so we need to do additional
        // initialization.
        info!("Detected new repo, initializing");
        mark_all_commits_as_old(conn).await?;
        track_main_branch(conn, repo).await?;
        info!("Initialized new repo");
    }

    update_tracked_refs(conn, repo).await?;

    tx.commit().await?;
    debug!("Finished adding new commits to db");
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

fn get_new_commits_from_repo(
    repo: &Repository,
    old: &HashSet<ObjectId>,
) -> anyhow::Result<Vec<Info>> {
    // Collect all references starting with "refs"
    let mut all_references: Vec<ObjectId> = vec![];
    for reference in repo.references()?.prefixed("refs")? {
        let reference = reference.map_err(|e| anyhow!(e))?;
        let id = reference.into_fully_peeled_id()?;

        // Some repos *cough*linuxkernel*cough* have refs that don't point to
        // commits. This makes the rev walk choke and die. We don't want that.
        if id.object()?.kind != Kind::Commit {
            continue;
        }

        all_references.push(id.into());
    }

    // Walk from those until hitting old references
    let new_commits = repo
        .rev_walk(all_references)
        .selected(|c| !old.contains(c))?
        .map(|r| r.map(|i| i.detach()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(new_commits)
}

async fn insert_new_commits(conn: &mut SqliteConnection, new: &[Info]) -> anyhow::Result<()> {
    for commit in new {
        let hash = commit.id.to_string();
        sqlx::query!("INSERT OR IGNORE INTO commits (hash) VALUES (?)", hash)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn insert_new_commit_links(conn: &mut SqliteConnection, new: &[Info]) -> anyhow::Result<()> {
    for commit in new {
        let child = commit.id.to_string();
        for parent in &commit.parent_ids {
            let parent = parent.to_string();
            // Commits *cough*linuxkernel*cough* may list the same parent
            // multiple times, so we just ignore duplicates during insert.
            sqlx::query!(
                "INSERT OR IGNORE INTO commit_links (parent, child) VALUES (?, ?)",
                parent,
                child,
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

async fn mark_all_commits_as_old(conn: &mut SqliteConnection) -> anyhow::Result<()> {
    sqlx::query!("UPDATE commits SET new = 0")
        .execute(conn)
        .await?;
    Ok(())
}

async fn track_main_branch(conn: &mut SqliteConnection, repo: &Repository) -> anyhow::Result<()> {
    let Some(head) = repo.head_ref()? else { return Ok(()); };
    let name = head.inner.name.to_string();
    let hash = head.into_fully_peeled_id()?.to_string();
    sqlx::query!(
        "INSERT OR IGNORE INTO tracked_refs (name, hash) VALUES (?, ?)",
        name,
        hash,
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn update_tracked_refs(conn: &mut SqliteConnection, repo: &Repository) -> anyhow::Result<()> {
    let tracked_refs = sqlx::query!("SELECT name, hash FROM tracked_refs")
        .fetch_all(&mut *conn)
        .await?;

    for tracked_ref in tracked_refs {
        if let Some(reference) = repo.try_find_reference(&tracked_ref.name)? {
            let hash = reference.id().to_string();
            if hash != tracked_ref.hash {
                debug!("Updated tracked ref {}", tracked_ref.name);
                sqlx::query!(
                    "UPDATE tracked_refs SET hash = ? WHERE name = ?",
                    hash,
                    tracked_ref.name
                )
                .execute(&mut *conn)
                .await?;
            }
        } else {
            debug!("Deleted tracked ref {}", tracked_ref.name);
            sqlx::query!("DELETE FROM tracked_refs WHERE name = ?", tracked_ref.name)
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(())
}

async fn update_repo(state: &AppState) -> anyhow::Result<()> {
    info!("Updating repo");
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
                error!("Error updating repo:\n{e:?}");
            }
        }
        .instrument(debug_span!("update repo"))
        .await;

        tokio::time::sleep(state.config.repo.update_delay).await;
    }
}
