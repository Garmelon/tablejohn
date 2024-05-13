use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use maud::{html, Markup};
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    primitive::{Reachable, Timestamp},
    server::{
        format,
        web::{
            components,
            page::{Page, Tab},
            paths::{
                PathAdminQueueAddBatch, PathAdminQueueDecrease, PathAdminQueueDelete,
                PathAdminQueueIncrease, PathQueue, PathQueueDelete, PathQueueInner,
            },
            r#static::QUEUE_JS,
            server_config_ext::{AbsPath, ServerConfigExt},
        },
        workers::{WorkerInfo, Workers},
    },
    shared::WorkerStatus,
    somehow,
};

enum Status {
    Idle,
    Busy,
    Working(Markup),
}

struct Worker {
    link: Markup,
    status: Status,
}

struct Task {
    link_delete: AbsPath,
    link_increase: AbsPath,
    link_decrease: AbsPath,
    hash: String,
    commit: Markup,
    since: String,
    priority: i64,
    workers: Vec<Markup>,
    odd: bool,
}

fn sorted_workers(workers: &Mutex<Workers>) -> Vec<(String, WorkerInfo)> {
    let mut workers = workers
        .lock()
        .unwrap()
        .clean()
        .get_all()
        .into_iter()
        .collect::<Vec<_>>();
    workers.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    workers
}

async fn get_workers(
    config: &ServerConfig,
    db: &SqlitePool,
    workers: &[(String, WorkerInfo)],
) -> somehow::Result<Vec<Worker>> {
    let mut result = vec![];
    for (name, info) in workers {
        let status = match &info.status {
            WorkerStatus::Idle => Status::Idle,
            WorkerStatus::Busy => Status::Busy,
            WorkerStatus::Working(run) => {
                let message =
                    sqlx::query_scalar!("SELECT message FROM commits WHERE hash = ?", run.hash)
                        .fetch_one(db)
                        .await?;
                Status::Working(components::link_run_short(
                    config,
                    run.id.clone(),
                    &run.hash,
                    &message,
                ))
            }
        };

        result.push(Worker {
            link: components::link_worker(config, name.clone()),
            status,
        })
    }
    Ok(result)
}

async fn get_queue_data(
    config: &ServerConfig,
    db: &SqlitePool,
    workers: &[(String, WorkerInfo)],
) -> somehow::Result<Vec<Task>> {
    // Group workers by commit hash
    let mut workers_by_commit: HashMap<String, Vec<Markup>> = HashMap::new();
    for (name, info) in workers {
        if let WorkerStatus::Working(run) = &info.status {
            workers_by_commit
                .entry(run.hash.clone())
                .or_default()
                .push(components::link_worker(config, name.clone()));
        }
    }

    let mut tasks = sqlx::query!(
        r#"
        SELECT
            hash,
            message,
            reachable AS "reachable: Reachable",
            date AS "date: Timestamp",
            priority
        FROM queue
        JOIN commits USING (hash)
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC
        "#
    )
    .fetch(db)
    .map_ok(|r| Task {
        workers: workers_by_commit.remove(&r.hash).unwrap_or_default(),
        link_delete: config.path(PathQueueDelete {
            hash: r.hash.clone(),
        }),
        link_increase: config.path(PathAdminQueueIncrease {}),
        link_decrease: config.path(PathAdminQueueDecrease {}),
        hash: r.hash.clone(),
        commit: components::link_commit(config, r.hash, &r.message, r.reachable),
        since: format::delta_from_now(r.date),
        priority: r.priority,
        odd: false,
    })
    .try_collect::<Vec<_>>()
    .await?;

    let mut last_priority = None;
    let mut odd = false;
    for task in tasks.iter_mut().rev() {
        if last_priority.is_some() && last_priority != Some(task.priority) {
            odd = !odd;
        }
        task.odd = odd;
        last_priority = Some(task.priority);
    }

    Ok(tasks)
}

fn page_inner(workers: Vec<Worker>, tasks: Vec<Task>) -> Markup {
    html! {
        h2 { "Workers" }
        @if workers.is_empty() {
            p { "No workers connected" }
        } @else {
            table .queue-workers {
                thead {
                    tr {
                        th { "worker" }
                        th { "status" }
                    }
                }
                tbody {
                    @for worker in workers { tr {
                        td { (worker.link) }
                        td { @match worker.status {
                            Status::Idle => "idle",
                            Status::Busy => "busy",
                            Status::Working(link) => (link),
                        } }
                    } }
                }
            }
        }
        h2 { "Queue (" (tasks.len()) ")" }
        form .queue-commits method="post" {
            table #queue data-count=(tasks.len()) {
                thead {
                    tr {
                        th { "commit" }
                        th { "since" }
                        th { "priority" }
                        th { "worker" }
                    }
                }
                tbody {
                    @for task in tasks { tr .odd[task.odd] {
                        td { (task.commit) }
                        td {
                            (task.since) " ["
                            a href=(task.link_delete) title="Delete from queue" { "del" }
                            "]"
                        }
                        td {
                            (task.priority) " ["
                            button .linkish title="Increase priority by 1" formaction=(task.link_increase) name="hash" value=(task.hash) { "inc" }
                            "/"
                            button .linkish title="Decrease priority by 1" formaction=(task.link_decrease) name="hash" value=(task.hash) { "dec" }
                            "]"
                            td {
                                @if task.workers.is_empty() {
                                    "-"
                                }
                                (components::join(&task.workers, html! { ", " }))
                            }
                        }
                    } }
                }
            }
        }
    }
}

pub async fn get_queue_inner(
    _path: PathQueueInner,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(config, &db, &sorted_workers).await?;
    let tasks = get_queue_data(config, &db, &sorted_workers).await?;
    Ok(page_inner(workers, tasks))
}

pub async fn get_queue(
    _path: PathQueue,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(config, &db, &sorted_workers).await?;
    let tasks = get_queue_data(config, &db, &sorted_workers).await?;

    let html = Page::new(config)
        .title(format!("queue ({})", tasks.len()))
        .tab(Tab::Queue)
        .head(html! {
            script type="module" src=(config.path(QUEUE_JS)) {}
        })
        .body(html!{
            div #inner { (page_inner(workers, tasks)) }
            form method="post" action=(config.path(PathAdminQueueAddBatch {})) {
                label {
                    "Batch size: "
                    input name="amount" type="number" value="10" min="1";
                } " "
                label {
                    "Priority: "
                    input #priority name="priority" type="number" value="-1" min="-2147483648" max="2147483647";
                } " "
                button { "Add batch to queue" }
            }
        })
        .build();

    Ok(html)
}

pub async fn get_queue_delete(
    path: PathQueueDelete,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let Some(r) = sqlx::query!(
        r#"
        SELECT
            hash,
            message,
            reachable AS "reachable: Reachable"
            FROM commits
        JOIN queue USING (hash)
        WHERE hash = ?
        "#,
        path.hash,
    )
    .fetch_optional(&db)
    .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let commit = components::link_commit(config, r.hash.clone(), &r.message, r.reachable);

    let html = Page::new(config)
        .title(format!("del {}", format::commit_short(&r.hash, &r.message)))
        .tab(Tab::Queue)
        .body(html! {
            h2 { "Delete commit from queue" }
            p { "You are about to delete this commit from the queue:" }
            p { (commit) }
            p { "All runs of this commit currently in progress will be aborted!" }
            form method="post" action=(config.path(PathAdminQueueDelete {})) {
                input name="hash" type="hidden" value=(r.hash);
                button { "Delete commit and abort runs" }
            }
        })
        .build();

    Ok(html.into_response())
}
