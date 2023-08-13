use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use askama::Template;
use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{
    config::Config,
    server::{
        util,
        workers::{WorkerInfo, Workers},
    },
    shared::WorkerStatus,
    somehow,
};

use super::{
    link::{CommitLink, RunLink, WorkerLink},
    paths::{PathQueue, PathQueueInner},
    Base, Tab,
};

enum Status {
    Idle,
    Busy,
    Working(RunLink),
}

struct Worker {
    link: WorkerLink,
    status: Status,
}

struct Task {
    commit: CommitLink,
    since: String,
    priority: i64,
    workers: Vec<WorkerLink>,
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
            WorkerStatus::Working(unfinished) => {
                let message = sqlx::query_scalar!(
                    "SELECT message FROM commits WHERE hash = ?",
                    unfinished.run.hash
                )
                .fetch_one(db)
                .await?;
                Status::Working(RunLink::new(
                    base,
                    unfinished.run.id.clone(),
                    &unfinished.run.hash,
                    &message,
                ))
            }
        };

        result.push(Worker {
            link: WorkerLink::new(base, name.clone()),
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
    let mut workers_by_commit: HashMap<String, Vec<WorkerLink>> = HashMap::new();
    for (name, info) in workers {
        if let WorkerStatus::Working(unfinished) = &info.status {
            workers_by_commit
                .entry(unfinished.run.hash.clone())
                .or_default()
                .push(WorkerLink::new(base, name.clone()));
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
        commit: CommitLink::new(base, r.hash, &r.message, r.reachable),
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
#[template(path = "queue_inner.html")]
struct QueueInnerTemplate {
    workers: Vec<Worker>,
    tasks: Vec<Task>,
}

pub async fn get_queue_inner(
    _path: PathQueueInner,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(&db, &sorted_workers, &base).await?;
    let tasks = get_queue_data(&db, &sorted_workers, &base).await?;
    Ok(QueueInnerTemplate { workers, tasks })
}
#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    base: Base,
    inner: QueueInnerTemplate,
}

pub async fn get_queue(
    _path: PathQueue,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_workers = sorted_workers(&workers);
    let workers = get_workers(&db, &sorted_workers, &base).await?;
    let tasks = get_queue_data(&db, &sorted_workers, &base).await?;
    Ok(QueueTemplate {
        base,
        inner: QueueInnerTemplate { workers, tasks },
    })
}
