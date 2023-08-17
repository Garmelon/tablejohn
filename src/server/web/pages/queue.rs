use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
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

#[derive(Template)]
#[template(path = "pages/queue_inner.html")]
struct PageInner {
    link_admin_queue_add_batch: Link,
    workers: Vec<Worker>,
    tasks: Vec<Task>,
}

pub async fn get_queue_inner(
    _path: PathQueueInner,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    Ok(PageInner {
        link_admin_queue_add_batch: base.link(PathAdminQueueAddBatch {}),
        workers: get_workers(&db, &sorted_workers, &base).await?,
        tasks: get_queue_data(&db, &sorted_workers, &base).await?,
    })
}

#[derive(Template)]
#[template(path = "pages/queue.html")]
struct Page {
    link_queue_js: Link,
    base: Base,
    inner: PageInner,
}

pub async fn get_queue(
    _path: PathQueue,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    Ok(Page {
        link_queue_js: base.link(QUEUE_JS),
        inner: PageInner {
            link_admin_queue_add_batch: base.link(PathAdminQueueAddBatch {}),
            workers: get_workers(&db, &sorted_workers, &base).await?,
            tasks: get_queue_data(&db, &sorted_workers, &base).await?,
        },
        base,
    })
}

#[derive(Template)]
#[template(path = "pages/queue_delete.html")]
struct PageDelete {
    base: Base,
    link_delete: Link,

    short: String,
    commit: LinkCommit,
    hash: String,
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

    Ok(PageDelete {
        short: util::format_commit_short(&r.hash, &r.message),
        commit: LinkCommit::new(&base, r.hash.clone(), &r.message, r.reachable),
        hash: r.hash,

        link_delete: base.link(PathAdminQueueDelete {}),
        base,
    }
    .into_response())
}
