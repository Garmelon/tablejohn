mod util;

use std::collections::HashMap;

use askama::Template;
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::extract::Query;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, SqlitePool};
use time::OffsetDateTime;

use crate::{
    config::Config,
    server::web::{
        base::{Base, Link, Tab},
        paths::{PathGraph, PathGraphData},
        r#static::{GRAPH_JS, UPLOT_CSS},
    },
    somehow,
};

// TODO Metric tree selector in template
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
    parents: HashMap<usize, Vec<usize>>,
    times: Vec<i64>,

    // TODO f32 for smaller transmission size?
    measurements: HashMap<String, Vec<Option<f64>>>,
}

pub async fn get_graph_data(
    _path: PathGraphData,
    State(db): State<SqlitePool>,
    Query(form): Query<QueryGraphData>,
) -> somehow::Result<impl IntoResponse> {
    let mut tx = db.begin().await?;
    let conn = tx.acquire().await?;

    // The SQL queries that return one result per commit *must* return the same
    // amount of rows in the same order!

    let unsorted_hashes = sqlx::query_scalar!(
        "\
        SELECT hash FROM commits \
        ORDER BY unixepoch(committer_date) ASC, hash ASC \
        "
    )
    .fetch_all(&mut *conn)
    .await?;

    let parent_child_pairs = sqlx::query!(
        "\
        SELECT parent, child \
        FROM commit_links \
        JOIN commits AS p ON p.hash = parent \
        JOIN commits AS c ON c.hash = child \
        ORDER BY \
            unixepoch(p.committer_date) ASC, p.hash ASC, \
            unixepoch(c.committer_date) ASC, c.hash ASC \
        "
    )
    .fetch(&mut *conn)
    .map_ok(|r| (r.parent, r.child))
    .try_collect::<Vec<_>>()
    .await?;

    let sorted_hashes = util::sort_topologically(&unsorted_hashes, &parent_child_pairs);

    let sorted_hash_indices = sorted_hashes
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, hash)| (hash, i))
        .collect::<HashMap<_, _>>();

    let mut parents = HashMap::<usize, Vec<usize>>::new();
    for (parent, child) in &parent_child_pairs {
        let parent_idx = sorted_hash_indices[parent];
        let child_idx = sorted_hash_indices[child];
        parents.entry(parent_idx).or_default().push(child_idx);
    }

    // permutation[unsorted_index] = sorted_index
    let permutation = unsorted_hashes
        .iter()
        .map(|h| sorted_hash_indices[h])
        .collect::<Vec<_>>();

    // Collect and permutate commit times
    let mut times = vec![0; sorted_hashes.len()];
    let mut rows = sqlx::query_scalar!(
        "\
        SELECT committer_date AS \"time: OffsetDateTime\" FROM commits \
        ORDER BY unixepoch(committer_date) ASC, hash ASC \
        "
    )
    .fetch(&mut *conn)
    .enumerate();
    while let Some((i, time)) = rows.next().await {
        times[permutation[i]] = time?.unix_timestamp();
    }
    drop(rows);

    // Collect and permutate measurements
    let mut measurements = HashMap::new();
    for metric in form.metric {
        let mut values = vec![None; sorted_hashes.len()];
        let mut rows = sqlx::query_scalar!(
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
        .fetch(&mut *conn)
        .enumerate();
        while let Some((i, value)) = rows.next().await {
            values[permutation[i]] = value?;
        }
        drop(rows);

        measurements.insert(metric, values);
    }

    Ok(Json(GraphData {
        hashes: sorted_hashes,
        parents,
        times,
        measurements,
    }))
}
