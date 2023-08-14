use std::collections::HashMap;

use askama::Template;
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqlitePool};

use crate::{
    config::Config,
    server::web::{
        base::{Base, Link, Tab},
        paths::{PathGraph, PathGraphData},
        r#static::{GRAPH_JS, UPLOT_CSS},
    },
    somehow,
};

#[derive(Template)]
#[template(path = "pages/graph.html")]
struct Page {
    link_uplot_css: Link,
    link_graph_js: Link,
    base: Base,
}

pub async fn get_graph(
    _path: PathGraph,
    State(config): State<&'static Config>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Graph);
    Ok(Page {
        link_uplot_css: base.link(UPLOT_CSS),
        link_graph_js: base.link(GRAPH_JS),
        base,
    })
}

#[derive(Deserialize)]
pub struct QueryGraphData {
    metric: Vec<String>,
}

#[derive(Serialize)]
struct GraphData {
    hashes: Vec<String>,
    times: Vec<i64>,
    metrics: HashMap<String, Vec<Option<f64>>>,
}

pub async fn get_graph_data(
    _path: PathGraphData,
    State(db): State<SqlitePool>,
    Query(form): Query<QueryGraphData>,
) -> somehow::Result<impl IntoResponse> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let rows = sqlx::query!(
        "\
        SELECT \
            hash, \
            committer_date AS \"committer_date: time::OffsetDateTime\" \
        FROM commits \
        ORDER BY unixepoch(committer_date) ASC, hash ASC \
        "
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut hashes = Vec::with_capacity(rows.len());
    let mut times = Vec::with_capacity(rows.len());
    for row in rows {
        hashes.push(row.hash);
        times.push(row.committer_date.unix_timestamp());
    }

    // TODO Somehow sort topologically if committer_date is the same
    // TODO Overhaul indices once I know how the query looks
    let mut metrics = HashMap::new();
    for metric in form.metric {
        let values = sqlx::query_scalar!(
            "\
            WITH \
            measurements AS ( \
                SELECT hash, value, MAX(start) \
                FROM runs \
                JOIN run_measurements USING (id) \
                WHERE name = ? \
                GROUP BY hash \
            ) \
            SELECT value \
            FROM commits \
            LEFT JOIN measurements USING (hash) \
            WHERE reachable = 2 \
            ORDER BY unixepoch(committer_date) ASC, hash ASC \
            ",
            metric,
        )
        .fetch_all(&mut *conn)
        .await?;

        metrics.insert(metric, values);
    }

    Ok(Json(GraphData {
        hashes,
        times,
        metrics,
    }))
}
