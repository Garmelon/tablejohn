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
    server::{
        util,
        web::{
            base::{Base, Link, Tab},
            link::{LinkCommit, LinkRunShort, LinkWorker},
            paths::{
                PathAdminQueueAddBatch, PathAdminQueueDecrease, PathAdminQueueDelete,
                PathAdminQueueIncrease, PathQueue, PathQueueDelete, PathQueueInner,
            },
            r#static::QUEUE_JS,
        },
        workers::{WorkerInfo, Workers},
    },
    shared::WorkerStatus,
    somehow,
};

enum Status {
    Idle,
    Busy,
    Working(LinkRunShort),
}

struct Worker {
    link: LinkWorker,
    status: Status,
}

struct Task {
    link_delete: Link,
    link_increase: Link,
    link_decrease: Link,
    hash: String,
    commit: LinkCommit,
    since: String,
    priority: i64,
    workers: Vec<LinkWorker>,
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
    db: &SqlitePool,
    workers: &[(String, WorkerInfo)],
    base: &Base,
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
                Status::Working(LinkRunShort::new(base, run.id.clone(), &run.hash, &message))
            }
        };

        result.push(Worker {
            link: LinkWorker::new(base, name.clone()),
            status,
        })
    }
    Ok(result)
}

async fn get_queue_data(
    db: &SqlitePool,
    workers: &[(String, WorkerInfo)],
    base: &Base,
) -> somehow::Result<Vec<Task>> {
    // Group workers by commit hash
    let mut workers_by_commit: HashMap<String, Vec<LinkWorker>> = HashMap::new();
    for (name, info) in workers {
        if let WorkerStatus::Working(run) = &info.status {
            workers_by_commit
                .entry(run.hash.clone())
                .or_default()
                .push(LinkWorker::new(base, name.clone()));
        }
    }

    let mut tasks = sqlx::query!(
        "\
        SELECT \
            hash, \
            message, \
            reachable, \
            date AS \"date: time::OffsetDateTime\", \
            priority \
        FROM queue \
        JOIN commits USING (hash) \
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC \
        "
    )
    .fetch(db)
    .map_ok(|r| Task {
        workers: workers_by_commit.remove(&r.hash).unwrap_or_default(),
        link_delete: base.link(PathQueueDelete {
            hash: r.hash.clone(),
        }),
        link_increase: base.link(PathAdminQueueIncrease {}),
        link_decrease: base.link(PathAdminQueueDecrease {}),
        hash: r.hash.clone(),
        commit: LinkCommit::new(base, r.hash, &r.message, r.reachable),
        since: util::format_delta_from_now(r.date),
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
                        td { (worker.link.html()) }
                        td { @match worker.status {
                            Status::Idle => "idle",
                            Status::Busy => "busy",
                            Status::Working(link) => (link.html()),
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
                        td { (task.commit.html()) }
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
                                @for (i, worker) in task.workers.iter().enumerate() {
                                    @if i > 0 { ", " }
                                    (worker.html())
                                }
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
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(&db, &sorted_workers, &base).await?;
    let tasks = get_queue_data(&db, &sorted_workers, &base).await?;
    Ok(page_inner(workers, tasks))
}

pub async fn get_queue(
    _path: PathQueue,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(&db, &sorted_workers, &base).await?;
    let tasks = get_queue_data(&db, &sorted_workers, &base).await?;

    Ok(base.html(
        &format!("queue ({})", tasks.len()),
        html! {
            script type="module" src=(base.link(QUEUE_JS)) {}
        },
        html! {
            div #inner { (page_inner(workers, tasks)) }
            form method="post" action=(base.link(PathAdminQueueAddBatch {})) {
                label {
                    "Batch size: "
                    input name="amount" type="number" value="10" min="1";
                }
                " "
                label {
                    "Priority: "
                    input #priority name="priority" type="number" value="-1" min="-2147483648" max="2147483647";
                }
                " "
                button { "Add batch to queue" }
            }
        },
    ))
}

pub async fn get_queue_delete(
    path: PathQueueDelete,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let base = Base::new(config, Tab::Queue);

    let Some(r) = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN queue USING (hash) \
        WHERE hash = ? \
        ",
        path.hash,
    )
    .fetch_optional(&db)
    .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let commit = LinkCommit::new(&base, r.hash.clone(), &r.message, r.reachable);

    Ok(base
        .html(
            &format!("del {}", util::format_commit_short(&r.hash, &r.message)),
            html! {},
            html! {
                h2 { "Delete commit from queue" }
                p { "You are about to delete this commit from the queue:" }
                p { (commit.html()) }
                p { "All runs of this commit currently in progress will be aborted!" }
                form method="post" action=(base.link(PathAdminQueueDelete {})) {
                    input name="hash" type="hidden" value=(r.hash);
                    button { "Delete commit and abort runs" }
                }
            },
        )
        .into_response())
}
