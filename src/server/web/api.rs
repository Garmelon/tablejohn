mod auth;

use std::sync::{Arc, Mutex};

use askama_axum::{IntoResponse, Response};
use axum::{
    extract::State,
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    routing::post,
    Json, Router, TypedHeader,
};
use sqlx::SqlitePool;
use time::OffsetDateTime;
use tracing::debug;

use crate::{
    config::Config,
    server::{
        runners::{RunnerInfo, Runners},
        BenchRepo, Server,
    },
    shared::{BenchMethod, RunnerRequest, ServerResponse, Work},
    somehow,
};

async fn post_status(
    auth: Option<TypedHeader<Authorization<Basic>>>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(bench_repo): State<Option<BenchRepo>>,
    State(runners): State<Arc<Mutex<Runners>>>,
    Json(request): Json<RunnerRequest>,
) -> somehow::Result<Response> {
    let name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };

    let now = OffsetDateTime::now_utc();
    let queue = sqlx::query_scalar!(
        "\
        SELECT hash FROM queue \
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC \
        "
    )
    .fetch_all(&db)
    .await?;

    let mut guard = runners.lock().unwrap();
    guard.clean(now);
    if !guard.verify(&name, &request.secret) {
        return Ok((StatusCode::UNAUTHORIZED, "invalid secret").into_response());
    }
    guard.update(
        name.clone(),
        RunnerInfo::new(request.secret, now, request.status),
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
            Some(bench_repo) => BenchMethod::BenchRepo {
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

pub fn router(server: &Server) -> Router<Server> {
    if server.repo.is_none() {
        return Router::new();
    }

    // TODO Get repo tar
    // TODO Get bench repo tar
    Router::new().route("/api/runner/status", post(post_status))
}
