mod auth;
mod stream;

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use gix::{ObjectId, ThreadSafeRepository};
use log::{debug, info};
use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;

use crate::{
    config::ServerConfig,
    primitive::Timestamp,
    server::{
        web::paths::{
            PathApiWorkerBenchRepoByHashTreeTarGz, PathApiWorkerRepoByHashTreeTarGz,
            PathApiWorkerStatus,
        },
        workers::{WorkerInfo, Workers},
        BenchRepo, Repo,
    },
    shared::{BenchMethod, FinishedRun, ServerResponse, WorkerRequest},
    somehow,
};

async fn save_work(
    run: FinishedRun,
    worker_name: &str,
    worker_info: &Option<String>,
    db: &SqlitePool,
) -> somehow::Result<()> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let end = run.end.map(|t| t.0).unwrap_or_else(OffsetDateTime::now_utc);

    sqlx::query!(
        "
        INSERT INTO runs (
            id,
            hash,
            bench_method,
            worker_name,
            worker_info,
            start,
            end,
            exit_code
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ",
        run.id,
        run.hash,
        run.bench_method,
        worker_name,
        worker_info,
        run.start.0,
        end,
        run.exit_code,
    )
    .execute(&mut *conn)
    .await?;

    // Now that we know the commit exists, we can defer all other foreign key
    // checks until the end of the transaction to improve insert performance.
    sqlx::query!("PRAGMA defer_foreign_keys=1")
        .execute(&mut *conn)
        .await?;

    for (metric, measurement) in run.measurements {
        sqlx::query!(
            "
            INSERT OR REPLACE INTO metrics (name, unit)
            VALUES (?, ?)
            ",
            metric,
            measurement.unit,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query!(
            "
            INSERT INTO run_measurements (
                id,
                metric,
                value,
                unit
            )
            VALUES (?, ?, ?, ?)
            ",
            run.id,
            metric,
            measurement.value,
            measurement.unit,
        )
        .execute(&mut *conn)
        .await?;
    }

    for (line, (source, text)) in run.output.into_iter().enumerate() {
        // Hopefully we won't need more than 4294967296 lines per run :P
        let line = line as u32;
        sqlx::query!(
            "
            INSERT INTO run_output (
                id,
                line,
                source,
                text
            )
            VALUES (?, ?, ?, ?)
            ",
            run.id,
            line,
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

pub async fn post_api_worker_status(
    _path: PathApiWorkerStatus,
    State(config): State<&'static ServerConfig>,
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
    debug!("Received status update from {name}");

    if let Some(run) = request.submit_run {
        info!("Received run {} for {} from {name}", run.id, run.hash);
        save_work(run, &name, &request.info, &db).await?;
    }

    // Fetch queue
    let queue = sqlx::query_scalar!(
        "\
        SELECT hash FROM queue \
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC \
        "
    )
    .fetch_all(&db)
    .await?;

    // Fetch bench method
    let bench_method = match bench_repo {
        Some(bench_repo) => BenchMethod::Repo {
            hash: bench_repo.0.to_thread_local().head_id()?.to_string(),
        },
        None => BenchMethod::Internal,
    };

    // Update internal state
    let (work, abort_work) = {
        let mut guard = workers.lock().unwrap();
        guard.clean();
        if !guard.verify_secret(&name, &request.secret) {
            return Ok((StatusCode::UNAUTHORIZED, "invalid secret").into_response());
        }
        guard.update(
            name.clone(),
            WorkerInfo::new(request.secret, Timestamp::now(), request.status),
        );
        let work = match request.request_run {
            true => guard.find_and_reserve_run(&name, &queue, bench_method),
            false => None,
        };
        let abort_work = guard.should_abort_work(&name, &queue);
        (work, abort_work)
    };

    Ok(Json(ServerResponse {
        run: work,
        abort_run: abort_work,
    })
    .into_response())
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
        Body::from_stream(stream::tar_and_gzip(repo, id)),
    )
}

pub async fn get_api_worker_repo_by_hash_tree_tar_gz(
    path: PathApiWorkerRepoByHashTreeTarGz,
    State(config): State<&'static ServerConfig>,
    State(repo): State<Option<Repo>>,
    auth: Option<TypedHeader<Authorization<Basic>>>,
) -> somehow::Result<Response> {
    let name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };
    debug!("Worker {name} is downloading repo hash {}", path.hash);

    let Some(repo) = repo else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let id = path.hash.parse::<ObjectId>()?;
    Ok(stream_response(repo.0, id).into_response())
}

pub async fn get_api_worker_bench_repo_by_hash_tree_tar_gz(
    path: PathApiWorkerBenchRepoByHashTreeTarGz,
    State(config): State<&'static ServerConfig>,
    State(bench_repo): State<Option<BenchRepo>>,
    auth: Option<TypedHeader<Authorization<Basic>>>,
) -> somehow::Result<Response> {
    let name = match auth::authenticate(config, auth) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };
    debug!("Worker {name} is downloading bench repo hash {}", path.hash);

    let Some(bench_repo) = bench_repo else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let id = path.hash.parse::<ObjectId>()?;
    Ok(stream_response(bench_repo.0, id).into_response())
}
