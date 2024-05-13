use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use maud::html;
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::{
        format,
        web::{
            components,
            page::Page,
            paths::{PathAdminQueueAdd, PathCommitByHash},
            server_config_ext::ServerConfigExt,
        },
    },
    somehow,
};

pub async fn get_commit_by_hash(
    path: PathCommitByHash,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let Some(commit) = sqlx::query!(
        "\
        SELECT \
            hash, \
            author, \
            author_date AS \"author_date: time::OffsetDateTime\", \
            committer, \
            committer_date AS \"committer_date: time::OffsetDateTime\", \
            message, \
            reachable \
        FROM commits \
        WHERE hash = ? \
        ",
        path.hash,
    )
    .fetch_optional(&db)
    .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let parents = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_edges ON hash = parent \
        WHERE child = ? \
        ORDER BY reachable DESC, unixepoch(committer_date) ASC \
        ",
        path.hash,
    )
    .fetch(&db)
    .map_ok(|r| components::link_commit(config, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let children = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_edges ON hash = child \
        WHERE parent = ? \
        ORDER BY reachable DESC, unixepoch(committer_date) ASC \
        ",
        path.hash,
    )
    .fetch(&db)
    .map_ok(|r| components::link_commit(config, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let runs = sqlx::query!(
        "\
        SELECT \
            id, \
            start AS \"start: time::OffsetDateTime\" \
        FROM runs WHERE hash = ? \
        ",
        path.hash
    )
    .fetch(&db)
    .map_ok(|r| components::link_run_date(config, r.id, r.start))
    .try_collect::<Vec<_>>()
    .await?;

    let (class, title) = components::commit_class_and_title(commit.reachable);

    let html = Page::new(config)
        .title(format::commit_summary(&commit.message))
        .body(html! {
            h2 { "Commit" }
            div .commit-like .commit {
                span .title { "commit " (commit.hash) }
                dl {
                    dt { "Author:" }
                    dd { (commit.author) }

                    dt { "AuthorDate:" }
                    dd { (format::time(commit.author_date)) }

                    dt { "Commit:" }
                    dd { (commit.committer) }

                    dt { "CommitDate:" }
                    dd { (format::time(commit.committer_date)) }

                    @for commit in parents {
                        dt { "Parent:" }
                        dd { (commit) }
                    }

                    @for commit in children {
                        dt { "Child:" }
                        dd { (commit) }
                    }
                }
                pre .(class) title=(title) {
                    (commit.message.trim_end())
                }
            }
        })
        .body(html!{
            h2 { "Runs" }
            @if runs.is_empty() {
                p { "There aren't any runs yet." }
            } @else {
                ul {
                    @for run in runs {
                        li { (run) }
                    }
                }
            }
            form method="post" action=(config.path(PathAdminQueueAdd {})) {
                input type="hidden" name="hash" value=(commit.hash);
                button { "Add to queue" } " with a "
                label for="priority" { "priority" } " of "
                input id="priority" name="priority" type="number" value="10" min="-2147483648" max="2147483647";
                "."
            }
        })
        .build();

    Ok(html.into_response())
}
