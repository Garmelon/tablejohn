use std::collections::HashMap;

use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::Query;
use futures::TryStreamExt;
use maud::html;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqlitePool};

use crate::{
    config::ServerConfig,
    server::{
        util,
        web::{
            page::{Page, Tab},
            paths::{PathGraph, PathGraphCommits, PathGraphMeasurements, PathGraphMetrics},
            r#static::{GRAPH_JS, UPLOT_CSS},
            server_config_ext::ServerConfigExt,
        },
    },
    somehow,
};

pub async fn get_graph(
    _path: PathGraph,
    State(config): State<&'static ServerConfig>,
) -> somehow::Result<impl IntoResponse> {
    let html = Page::new(config)
        .title("graph")
        .tab(Tab::Graph)
        .head(html! {
            link rel="stylesheet" href=(config.path(UPLOT_CSS));
            script type="module" src=(config.path(GRAPH_JS)) {}
        })
        .body(html! {
            h2 { "Graph" }
            div .graph-container {
                div #plot {}
                div #metrics .metrics-list { "Loading metrics..." }
            }
        })
        .build();

    Ok(html)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsResponse {
    data_id: i64,
    metrics: Vec<String>,
}

pub async fn get_graph_metrics(
    _path: PathGraphMetrics,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let metrics =
        sqlx::query_scalar!("SELECT DISTINCT metric FROM run_measurements ORDER BY metric ASC")
            .fetch_all(&db)
            .await?;

    Ok(Json(MetricsResponse {
        data_id: 0, // TODO Implement
        metrics,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitsResponse {
    graph_id: i64,
    hash_by_hash: Vec<String>,
    author_by_hash: Vec<String>,
    committer_date_by_hash: Vec<i64>,
    summary_by_hash: Vec<String>,
    child_parent_index_pairs: Vec<(usize, usize)>,
}

pub async fn get_graph_commits(
    _path: PathGraphCommits,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    let mut hash_by_hash = vec![];
    let mut author_by_hash = vec![];
    let mut committer_date_by_hash = vec![];
    let mut summary_by_hash = vec![];
    let mut child_parent_index_pairs = vec![];

    // Fetch main commit info
    let mut rows = sqlx::query!(
        "\
        SELECT \
            hash, \
            author, \
            committer_date AS \"committer_date: time::OffsetDateTime\", \
            message \
        FROM commits \
        WHERE reachable = 2 \
        ORDER BY hash ASC \
        "
    )
    .fetch(&mut *conn);
    while let Some(row) = rows.try_next().await? {
        hash_by_hash.push(row.hash);
        author_by_hash.push(row.author);
        committer_date_by_hash.push(row.committer_date.unix_timestamp());
        summary_by_hash.push(util::format_commit_summary(&row.message));
    }
    drop(rows);

    // Map from hash to index in "by hash" order
    let index_of_hash = hash_by_hash
        .iter()
        .cloned()
        .enumerate()
        .map(|(idx, hash)| (hash, idx))
        .collect::<HashMap<_, _>>();

    // Fetch parent info
    let mut rows = sqlx::query!(
        "\
        SELECT child, parent \
        FROM commit_links \
        JOIN commits ON hash = child \
        WHERE reachable = 2 \
        ORDER BY hash ASC \
        "
    )
    .fetch(&mut *conn);
    while let Some(row) = rows.try_next().await? {
        // The child is tracked and must thus be in our map.
        let child_index = *index_of_hash.get(&row.child).unwrap();

        // The parent of a tracked commit must also be tracked.
        let parent_index = *index_of_hash.get(&row.parent).unwrap();

        child_parent_index_pairs.push((child_index, parent_index));
    }
    drop(rows);

    Ok(Json(CommitsResponse {
        graph_id: 0, // TODO Implement
        hash_by_hash,
        author_by_hash,
        committer_date_by_hash,
        summary_by_hash,
        child_parent_index_pairs,
    }))
}

#[derive(Deserialize)]
pub struct QueryGraphMeasurements {
    #[serde(default)]
    metric: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeasurementsResponse {
    graph_id: i64,
    data_id: i64,
    measurements: HashMap<String, Vec<f64>>,
}

pub async fn get_graph_measurements(
    _path: PathGraphMeasurements,
    State(db): State<SqlitePool>,
    Query(form): Query<QueryGraphMeasurements>,
) -> somehow::Result<impl IntoResponse> {
    Ok(Json(MeasurementsResponse {
        graph_id: 0,                  // TODO Implement
        data_id: 0,                   // TODO Implement
        measurements: HashMap::new(), // TODO Implement
    }))
}
