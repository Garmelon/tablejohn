use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{
    config::Config,
    server::{
        util,
        web::{
            base::{Base, Tab},
            link::LinkCommit,
            paths::PathRunById,
        },
    },
    somehow,
};

struct Measurement {
    metric: String,
    value: String,
    stddev: String,
    unit: String,
    direction: &'static str,
}

struct Line {
    err: bool,
    text: String,
}

#[derive(Template)]
#[template(path = "pages/run_finished.html")]
struct PageFinished {
    base: Base,

    summary: String,
    id: String,
    commit: LinkCommit,
    bench_method: String,
    start: String,
    end: String,
    duration: String,
    exit_code: i64,
    measurements: Vec<Measurement>,
    output: Vec<Line>,
}

async fn from_finished_run(
    id: &str,
    config: &'static Config,
    db: &SqlitePool,
) -> somehow::Result<Option<Response>> {
    let Some(run) = sqlx::query!(
        "\
        SELECT \
            id, \
            hash, \
            bench_method, \
            start AS \"start: time::OffsetDateTime\", \
            end AS \"end: time::OffsetDateTime\", \
            exit_code, \
            message, \
            reachable \
        FROM runs \
        JOIN commits USING (hash) \
        WHERE id = ? \
        ",
        id,
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };

    let measurements = sqlx::query!(
        "\
        SELECT \
            metric, \
            value, \
            stddev, \
            unit, \
            direction \
        FROM run_measurements \
        WHERE id = ? \
        ORDER BY metric ASC \
        ",
        id,
    )
    .fetch(db)
    .map_ok(|r| Measurement {
        metric: r.metric,
        value: util::format_value(r.value),
        stddev: r.stddev.map(util::format_value).unwrap_or_default(),
        unit: r.unit.unwrap_or_default(),
        direction: match r.direction {
            Some(..=-1) => "less is better",
            Some(0) => "neutral",
            Some(1..) => "more is better",
            None => "",
        },
    })
    .try_collect::<Vec<_>>()
    .await?;

    let output = sqlx::query!(
        "\
        SELECT source, text FROM run_output \
        WHERE id = ? \
        ORDER BY idx ASC \
        ",
        id,
    )
    .fetch(db)
    .map_ok(|r| Line {
        err: r.source != 1,
        text: r.text,
    })
    .try_collect::<Vec<_>>()
    .await?;

    let base = Base::new(config, Tab::None);
    Ok(Some(
        PageFinished {
            summary: util::format_commit_summary(&run.message),
            id: run.id,
            commit: LinkCommit::new(&base, run.hash, &run.message, run.reachable),
            bench_method: run.bench_method,
            start: util::format_time(run.start),
            end: util::format_time(run.end),
            duration: util::format_duration(run.end - run.start),
            exit_code: run.exit_code,
            measurements,
            output,

            base,
        }
        .into_response(),
    ))
}

pub async fn get_run_by_id(
    path: PathRunById,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    if let Some(response) = from_finished_run(&path.id, config, &db).await? {
        Ok(response)
    } else {
        // TODO Show unfinished runs
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}
