//! Add new commits to the database and update the tracked refs.

use std::collections::HashSet;

use futures::TryStreamExt;
use gix::{objs::Kind, prelude::ObjectIdExt, refs::Reference, ObjectId, Repository};
use sqlx::{Acquire, SqliteConnection, SqlitePool};
use tracing::{debug, info};

use crate::{
    server::{util, Repo},
    somehow,
};

async fn get_all_commit_hashes_from_db(
    conn: &mut SqliteConnection,
) -> somehow::Result<HashSet<ObjectId>> {
    let hashes = sqlx::query!("SELECT hash FROM commits")
        .fetch(conn)
        .err_into::<somehow::Error>()
        .and_then(|r| async move { r.hash.parse::<ObjectId>().map_err(|e| e.into()) })
        .try_collect::<HashSet<_>>()
        .await?;

    Ok(hashes)
}

fn get_all_refs_from_repo(repo: &Repository) -> somehow::Result<Vec<Reference>> {
    let mut references = vec![];
    for reference in repo.references()?.all()? {
        let mut reference = reference.map_err(somehow::Error::from_box)?;
        reference.peel_to_id_in_place()?;

        // Some repos *cough*linuxkernel*cough* have refs that don't point to
        // commits. This makes the rev walk choke and die. We don't want that.
        if reference.id().object()?.kind != Kind::Commit {
            continue;
        }

        references.push(reference.detach());
    }
    Ok(references)
}

fn get_new_commits_from_repo(
    repo: &Repository,
    refs: &[Reference],
    old: &HashSet<ObjectId>,
) -> somehow::Result<Vec<ObjectId>> {
    let ref_ids = refs.iter().flat_map(|r| r.peeled.into_iter());

    // Walk from those until hitting old references
    let mut new = vec![];
    for commit in repo.rev_walk(ref_ids).selected(|c| !old.contains(c))? {
        new.push(commit?.id);
    }

    Ok(new)
}

fn get_all_refs_and_new_commits_from_repo(
    repo: &Repository,
    old: &HashSet<ObjectId>,
) -> somehow::Result<(Vec<Reference>, Vec<ObjectId>)> {
    let refs = get_all_refs_from_repo(repo)?;
    let new = get_new_commits_from_repo(repo, &refs, old)?;
    Ok((refs, new))
}

async fn insert_new_commits(
    conn: &mut SqliteConnection,
    repo: &Repository,
    new: &[ObjectId],
) -> somehow::Result<()> {
    for (i, id) in new.iter().enumerate() {
        let commit = id.attach(repo).object()?.try_into_commit()?;
        let hash = commit.id.to_string();
        let author_info = commit.author()?;
        let author = util::format_actor(author_info.actor())?;
        let author_date = util::time_to_offset_datetime(author_info.time)?;
        let committer_info = commit.committer()?;
        let committer = util::format_actor(committer_info.actor())?;
        let committer_date = util::time_to_offset_datetime(committer_info.time)?;
        let message = commit.message_raw()?.to_string();

        sqlx::query!(
            "\
            INSERT OR IGNORE INTO commits ( \
                hash, \
                author, \
                author_date, \
                committer, \
                committer_date, \
                message \
            ) \
            VALUES (?, ?, ?, ?, ?, ?) \
            ",
            hash,
            author,
            author_date,
            committer,
            committer_date,
            message
        )
        .execute(&mut *conn)
        .await?;

        if (i + 1) % 100000 == 0 {
            debug!("Inserted {} commits so far", i + 1);
        }
    }
    debug!("Inserted {} commits in total", new.len());
    Ok(())
}

async fn insert_new_commit_links(
    conn: &mut SqliteConnection,
    repo: &Repository,
    new: &[ObjectId],
) -> somehow::Result<()> {
    for (i, hash) in new.iter().enumerate() {
        let commit = hash.attach(repo).object()?.try_into_commit()?;
        let child = commit.id.to_string();
        for parent in commit.parent_ids() {
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

        if (i + 1) % 100000 == 0 {
            debug!("Inserted {} commits' links so far", i + 1);
        }
    }
    debug!("Inserted {} commits' links in total", new.len());
    Ok(())
}

async fn mark_all_commits_as_old(conn: &mut SqliteConnection) -> somehow::Result<()> {
    sqlx::query!("UPDATE commits SET new = 0")
        .execute(conn)
        .await?;
    Ok(())
}

async fn update_refs(conn: &mut SqliteConnection, refs: Vec<Reference>) -> somehow::Result<()> {
    // Remove refs that no longer exist
    let existing = refs
        .iter()
        .map(|r| r.name.to_string())
        .collect::<HashSet<_>>();
    let current = sqlx::query!("SELECT name FROM refs")
        .fetch_all(&mut *conn)
        .await?;
    for reference in current {
        if !existing.contains(&reference.name) {
            sqlx::query!("DELETE FROM refs WHERE name = ?", reference.name)
                .execute(&mut *conn)
                .await?;
        }
    }

    // Add new refs and update existing refs
    for reference in refs {
        let name = reference.name.to_string();
        let Some(hash) = reference.peeled else { continue; };
        let hash = hash.to_string();

        sqlx::query!(
            "\
            INSERT INTO refs (name, hash) VALUES (?, ?) \
            ON CONFLICT (name) DO UPDATE \
                SET hash = excluded.hash \
            ",
            name,
            hash
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

async fn track_main_branch(conn: &mut SqliteConnection, repo: &Repository) -> somehow::Result<()> {
    let Some(head) = repo.head_ref()? else { return Ok(()); };
    let name = head.inner.name.to_string();
    sqlx::query!("UPDATE refs SET tracked = true WHERE name = ?", name)
        .execute(conn)
        .await?;
    Ok(())
}

async fn update_commit_tracked_status(conn: &mut SqliteConnection) -> somehow::Result<()> {
    sqlx::query!(
        "\
        WITH RECURSIVE \
            tracked (hash) AS ( \
                SELECT hash FROM refs WHERE tracked \
                UNION \
                SELECT parent FROM commit_links \
                JOIN tracked ON hash = child \
            ), \
            reachable (hash) AS ( \
                SELECT hash FROM refs \
                UNION \
                SELECT hash FROM tracked \
                UNION \
                SELECT parent FROM commit_links \
                JOIN reachable ON hash = child \
            ) \
        UPDATE commits \
        SET reachable = CASE \
            WHEN hash IN tracked   THEN 2 \
            WHEN hash IN reachable THEN 1 \
            ELSE 0 \
        END \
"
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(super) async fn update(db: &SqlitePool, repo: Repo) -> somehow::Result<()> {
    debug!("Updating repo");
    let thread_local_repo = repo.0.to_thread_local();
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let old = get_all_commit_hashes_from_db(&mut *conn).await?;
    debug!("Loaded {} commits from the db", old.len());

    let repo_is_new = old.is_empty();
    if repo_is_new {
        info!("Initializing new repo");
    }

    // This can take a while for larger repos. Running it via spawn_blocking
    // keeps it from blocking the entire tokio worker.
    let (refs, new) = tokio::task::spawn_blocking(move || {
        get_all_refs_and_new_commits_from_repo(&repo.0.to_thread_local(), &old)
    })
    .await??;
    debug!("Found {} new commits in repo", new.len());

    // Defer foreign key checks until the end of the transaction to improve
    // insert performance.
    sqlx::query!("PRAGMA defer_foreign_keys=1")
        .execute(&mut *conn)
        .await?;

    // Inserts are grouped by table so sqlite can process them *a lot* faster
    // than if they were grouped by commit (insert commit and parents, then next
    // commit and so on).
    insert_new_commits(conn, &thread_local_repo, &new).await?;
    insert_new_commit_links(conn, &thread_local_repo, &new).await?;
    if repo_is_new {
        mark_all_commits_as_old(conn).await?;
    }

    update_refs(conn, refs).await?;
    if repo_is_new {
        track_main_branch(conn, &thread_local_repo).await?;
    }
    update_commit_tracked_status(conn).await?;
    debug!("Updated tracked refs");

    tx.commit().await?;
    if repo_is_new {
        info!("Initialized new repo");
    }
    debug!("Updated repo");
    Ok(())
}
