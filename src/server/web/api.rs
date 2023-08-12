mod auth;
mod stream;

use std::sync::{Arc, Mutex};

use axum::{
    body::StreamBody,
    extract::{Path, State},
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router, TypedHeader,
};
use gix::{ObjectId, ThreadSafeRepository};
use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;
use tracing::debug;

use crate::{
    config::Config,
    server::{
        workers::{WorkerInfo, Workers},
        BenchRepo, Repo, Server,
    },
    shared::{BenchMethod, FinishedRun, ServerResponse, Work, WorkerRequest},
    somehow,
};

async fn save_work(run: FinishedRun, db: SqlitePool) -> somehow::Result<()> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    sqlx::query!(
        "\
        INSERT INTO runs ( \
            id, \
            hash, \
            start, \
            end, \
            exit_code \
        ) \
        VALUES (?, ?, ?, ?, ?) \
        ",
        run.id,
        run.hash,
        run.start,
        run.end,
        run.exit_code,
    )
    .execute(&mut *conn)
    .await?;

    for (name, measurement) in run.measurements {
        sqlx::query!(
            "\
            INSERT INTO run_measurements ( \
                id, \
                name, \
                value, \
                stddev, \
                unit, \
                direction \
            ) \
            VALUES (?, ?, ?, ?, ?, ?) \
            ",
            run.id,
            name,
            measurement.value,
            measurement.stddev,
            measurement.unit,
            measurement.direction,
        )
        .execute(&mut *conn)
        .await?;
    }

    for (idx, (source, text)) in run.output.into_iter().enumerate() {
        // Hopefully we won't need more than 4294967296 output chunks per run :P
        let idx = idx as u32;
        sqlx::query!(
            "\
            INSERT INTO run_output ( \
                id, \
                idx, \
                source, \
                text \
            ) \
            VALUES (?, ?, ?, ?) \
            ",
            run.id,
            idx,
            source,
            text,
        )
        .execute(&mut *conn)
        .await?;
    }

    // The thing has been done :D
    sqlx::query!("DELETE FROM queue WHERE hash = ?", run.hash)
        .execute(&mut *conn)
        .await?;

    tx.commit().await?;
    Ok(())
}

fn prepare_work(
    work: Option<&str>,
    bench_repo: Option<BenchRepo>,
) -> somehow::Result<Option<Work>> {
    Ok(if let Some(hash) = work {
        let bench = match bench_repo {
            Some(bench_repo) => BenchMethod::Repo {
                hash: bench_repo.0.to_thread_local().head_id()?.to_string(),
            },
            None => BenchMethod::Internal,
        };
        Some(Work {
            hash: hash.to_string(),
            bench,
        })
    } else {
        None
    })
}

async fn post_status(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(bench_repo): State<Option<BenchRepo>>,
    State(workers): State<Arc<Mutex<Workers>>>,
    auth: Option<TypedHeader<Authorization<Basic>>>,
    Json(request): Json<WorkerRequest>,
) -> somehow::Result<Response> {
    let name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };

    let queue = sqlx::query_scalar!(
        "\
        SELECT hash FROM queue \
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC \
        "
    )
    .fetch_all(&db)
    .await?;

    let (work, abort_work) = {
        let mut guard = workers.lock().unwrap();
        guard.clean();
        if !guard.verify(&name, &request.secret) {
            return Ok((StatusCode::UNAUTHORIZED, "invalid secret").into_response());
        }
        guard.update(
            name.clone(),
            WorkerInfo::new(request.secret, OffsetDateTime::now_utc(), request.status),
        );
        let work = match request.request_work {
            true => guard.find_free_work(&queue),
            false => None,
        };
        let abort_work = guard.should_abort_work(&name);
        (work, abort_work)
    };

    if let Some(run) = request.submit_work {
        save_work(run, db).await?;
    }

    let work = prepare_work(work, bench_repo)?;
    // TODO Reserve this work
    debug!("Received status update from {name}");
    Ok(Json(ServerResponse { work, abort_work }).into_response())
}

fn stream_response(repo: Arc<ThreadSafeRepository>, id: ObjectId) -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/gzip"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"tree.tar.gz\""),
            ),
        ],
        StreamBody::new(stream::tar_and_gzip(repo, id)),
    )
}

async fn get_repo(
    State(config): State<&'static Config>,
    State(repo): State<Option<Repo>>,
    auth: Option<TypedHeader<Authorization<Basic>>>,
    Path(hash): Path<String>,
) -> somehow::Result<Response> {
    let _name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };

    let Some(repo) = repo else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let id = hash.parse::<ObjectId>()?;
    Ok(stream_response(repo.0, id).into_response())
}

async fn get_bench_repo(
    State(config): State<&'static Config>,
    State(bench_repo): State<Option<BenchRepo>>,
    auth: Option<TypedHeader<Authorization<Basic>>>,
    Path(hash): Path<String>,
) -> somehow::Result<Response> {
    let _name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };

    let Some(bench_repo) = bench_repo else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let id = hash.parse::<ObjectId>()?;
    Ok(stream_response(bench_repo.0, id).into_response())
}

pub fn router(server: &Server) -> Router<Server> {
    if server.repo.is_none() {
        return Router::new();
    }

    Router::new()
        .route("/api/worker/status", post(post_status))
        .route("/api/worker/repo/:hash/tree.tar.gz", get(get_repo))
        .route(
            "/api/worker/bench_repo/:hash/tree.tar.gz",
            get(get_bench_repo),
        )
}
