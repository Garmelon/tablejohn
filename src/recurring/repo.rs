//! Add new commits to the database and update the tracked refs.

// TODO Prevent some sync stuff from blocking the async stuff

use std::collections::HashSet;

use futures::TryStreamExt;
use gix::{
    actor::IdentityRef, date::time::format::ISO8601_STRICT, objs::Kind, refs::Reference, Commit,
    ObjectId, Repository,
};
use sqlx::{Acquire, SqliteConnection, SqlitePool};
use tracing::{debug, info};

use crate::somehow;

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

fn get_new_commits_from_repo<'a, 'b: 'a>(
    repo: &'a Repository,
    refs: &[Reference],
    old: &'b HashSet<ObjectId>,
) -> somehow::Result<Vec<Commit<'a>>> {
    let ref_ids = refs.iter().flat_map(|r| r.peeled.into_iter());

    // Walk from those until hitting old references
    let mut new = vec![];
    for commit in repo.rev_walk(ref_ids).selected(|c| !old.contains(c))? {
        let commit = commit?.id().object()?.try_into_commit()?;
        new.push(commit);
    }

    Ok(new)
}

pub fn format_actor(author: IdentityRef<'_>) -> somehow::Result<String> {
    let mut buffer = vec![];
    author.trim().write_to(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

async fn insert_new_commits(
    conn: &mut SqliteConnection,
    new: &[Commit<'_>],
) -> somehow::Result<()> {
    for commit in new {
        let hash = commit.id.to_string();
        let author_info = commit.author()?;
        let author = format_actor(author_info.actor())?;
        let author_date = author_info.time.format(ISO8601_STRICT);
        let committer_info = commit.committer()?;
        let committer = format_actor(committer_info.actor())?;
        let committer_date = committer_info.time.format(ISO8601_STRICT);
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
    }
    Ok(())
}

async fn insert_new_commit_links(
    conn: &mut SqliteConnection,
    new: &[Commit<'_>],
) -> somehow::Result<()> {
    for commit in new {
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
    }
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

pub async fn update(db: &SqlitePool, repo: &Repository) -> somehow::Result<()> {
    debug!("Updating repo");
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let old = get_all_commit_hashes_from_db(&mut *conn).await?;
    debug!("Loaded {} commits from the db", old.len());

    let repo_is_new = old.is_empty();
    if repo_is_new {
        info!("Initializing new repo");
    }

    let refs = get_all_refs_from_repo(repo)?;
    let new = get_new_commits_from_repo(repo, &refs, &old)?;
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
    if repo_is_new {
        mark_all_commits_as_old(conn).await?;
    }
    debug!("Inserted {} new commits into db", new.len());

    update_refs(conn, refs).await?;
    if repo_is_new {
        track_main_branch(conn, repo).await?;
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
