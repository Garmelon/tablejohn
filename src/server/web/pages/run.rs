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
        format,
        web::{components, page::Page, paths::PathRunById},
    },
    somehow,
};

struct Measurement {
    metric: String,
    value: String,
    unit: String,
}

struct Line {
    err: bool,
    text: String,
}

async fn from_finished_run(
    id: &str,
    config: &'static ServerConfig,
    db: &SqlitePool,
) -> somehow::Result<Option<Markup>> {
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
            unit \
        FROM run_measurements \
        WHERE id = ? \
        ORDER BY metric ASC \
        ",
        id,
    )
    .fetch(db)
    .map_ok(|r| Measurement {
        metric: r.metric,
        value: format::measurement_value(r.value),
        unit: r.unit.unwrap_or_default(),
    })
    .try_collect::<Vec<_>>()
    .await?;

    let output = sqlx::query!(
        "\
        SELECT source, text FROM run_output \
        WHERE id = ? \
        ORDER BY line ASC \
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

    let commit = components::link_commit(config, run.hash, &run.message, run.reachable);

    let html = Page::new(config)
        .title(format!("Run of {}", format::commit_summary(&run.message)))
        .body(html! {
            h2 { "Run" }
            div .commit-like .run {
                span .title { "run " (run.id) }
                dl {
                    dt { "Commit:" }
                    dd { (commit)}

                    dt { "Benchmark:" }
                    dd { (run.bench_method) }

                    dt { "Start:" }
                    dd { (format::time(run.start)) }

                    dt { "End:" }
                    dd { (format::time(run.end)) }

                    dt { "Duration:" }
                    dd { (format::duration(run.end - run.start)) }

                    dt { "Exit code:" }
                    dd { (run.exit_code) }
                }
            }
        })
        .body(html! {
            h2 { "Measurements" }
            table {
                thead {
                    tr {
                        th { "metric" }
                        th { "value" }
                        th { "unit" }
                    }
                }
                tbody {
                    @for mm in measurements { tr {
                        td { (mm.metric) }
                        td { (mm.value) }
                        td { (mm.unit) }
                    } }
                }
            }
        })
        .body(html! {
            h2 { "Output" }
            div .run-output {
                @for line in output {
                    pre { (line.text) }
                }
            }
        })
        .build();

    Ok(Some(html))
}

pub async fn get_run_by_id(
    path: PathRunById,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    if let Some(markup) = from_finished_run(&path.id, config, &db).await? {
        Ok(markup.into_response())
    } else {
        // TODO Show unfinished runs
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}
