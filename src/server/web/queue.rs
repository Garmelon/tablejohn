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
        runners::{RunnerInfo, Runners},
        util,
    },
    shared::RunnerStatus,
    somehow,
};

use super::{
    link::{CommitLink, RunLink, RunnerLink},
    Base, Tab,
};

enum Status {
    Idle,
    Busy,
    Working(RunLink),
}

struct Runner {
    link: RunnerLink,
    status: Status,
}

struct Task {
    commit: CommitLink,
    since: String,
    priority: i64,
    runners: Vec<RunnerLink>,
    odd: bool,
}

fn sorted_runners(runners: &Mutex<Runners>) -> Vec<(String, RunnerInfo)> {
    let mut runners = runners
        .lock()
        .unwrap()
        .clean()
        .get_all()
        .into_iter()
        .collect::<Vec<_>>();
    runners.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    runners
}

async fn get_runners(
    db: &SqlitePool,
    runners: &[(String, RunnerInfo)],
    base: &Base,
) -> somehow::Result<Vec<Runner>> {
    let mut result = vec![];
    for (name, info) in runners {
        let status = match &info.status {
            RunnerStatus::Idle => Status::Idle,
            RunnerStatus::Busy => Status::Busy,
            RunnerStatus::Working(run) => {
                let message =
                    sqlx::query_scalar!("SELECT message FROM commits WHERE hash = ?", run.hash)
                        .fetch_one(db)
                        .await?;
                Status::Working(RunLink::new(base, run.id.clone(), &run.hash, &message))
            }
        };

        result.push(Runner {
            link: RunnerLink::new(base, name.clone()),
            status,
        })
    }
    Ok(result)
}

async fn get_queue(
    db: &SqlitePool,
    runners: &[(String, RunnerInfo)],
    base: &Base,
) -> somehow::Result<Vec<Task>> {
    // Group runners by commit hash
    let mut runners_by_commit: HashMap<String, Vec<RunnerLink>> = HashMap::new();
    for (name, info) in runners {
        if let RunnerStatus::Working(run) = &info.status {
            runners_by_commit
                .entry(run.hash.clone())
                .or_default()
                .push(RunnerLink::new(base, name.clone()));
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
        runners: runners_by_commit.remove(&r.hash).unwrap_or_default(),
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
    runners: Vec<Runner>,
    tasks: Vec<Task>,
}

pub async fn get_inner(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(runners): State<Arc<Mutex<Runners>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_runners = sorted_runners(&runners);
    let runners = get_runners(&db, &sorted_runners, &base).await?;
    let tasks = get_queue(&db, &sorted_runners, &base).await?;
    Ok(QueueInnerTemplate { runners, tasks })
}
#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    base: Base,
    inner: QueueInnerTemplate,
}

pub async fn get(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(runners): State<Arc<Mutex<Runners>>>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Queue);
    let sorted_runners = sorted_runners(&runners);
    let runners = get_runners(&db, &sorted_runners, &base).await?;
    let tasks = get_queue(&db, &sorted_runners, &base).await?;
    Ok(QueueTemplate {
        base,
        inner: QueueInnerTemplate { runners, tasks },
    })
}
