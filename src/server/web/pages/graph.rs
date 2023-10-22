use std::collections::HashMap;

use askama::Template;
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::web::{
        base::{Base, Link, Tab},
        paths::{PathGraph, PathGraphCommits, PathGraphMeasurements, PathGraphMetrics},
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
    State(config): State<&'static ServerConfig>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Graph);
    Ok(Page {
        link_uplot_css: base.link(UPLOT_CSS),
        link_graph_js: base.link(GRAPH_JS),
        base,
    })
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
    message_by_hash: Vec<String>,
    parents_by_hash: Vec<Vec<String>>,
}

pub async fn get_graph_commits(
    _path: PathGraphCommits,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    Ok(Json(CommitsResponse {
        graph_id: 0,                    // TODO Implement
        hash_by_hash: vec![],           // TODO Implement
        author_by_hash: vec![],         // TODO Implement
        committer_date_by_hash: vec![], // TODO Implement
        message_by_hash: vec![],        // TODO Implement
        parents_by_hash: vec![],        // TODO Implement
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
