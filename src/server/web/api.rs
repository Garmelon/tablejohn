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
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tracing::debug;

use crate::{
    config::Config,
    server::{
        workers::{WorkerInfo, Workers},
        BenchRepo, Repo, Server,
    },
    shared::{BenchMethod, ServerResponse, Work, WorkerRequest},
    somehow,
};

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
    drop(guard);

    // TODO Insert finished work into DB

    // Find new work
    let work = if let Some(hash) = work {
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
    };

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
