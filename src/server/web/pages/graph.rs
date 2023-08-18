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
    config::ServerConfig,
    server::web::{
        base::{Base, Link, Tab},
        paths::{PathGraph, PathGraphData},
        r#static::{GRAPH_JS, UPLOT_CSS},
    },
    somehow,
};

use self::util::MetricFolder;

#[derive(Template)]
#[template(
    ext = "html",
    source = "
{% match self %}
  {% when MetricTree::File with { name, metric } %}
    <label><input type=\"checkbox\" name=\"{{ metric }}\"> {{ name }}</label>
  {% when MetricTree::Folder with { name, metric, children } %}
    {% if children.trees.is_empty() %}
      {% if let Some(metric) = metric %}
        <label><input type=\"checkbox\" name=\"{{ metric }}\"> {{ name }}/</label>
      {% endif %}
    {% else if let Some(metric) = metric %}
      <details>
        <summary><input type=\"checkbox\" name=\"{{ metric }}\"> {{ name }}/</summary>
        {{ children|safe }}
      </details>
    {% else %}
      <details class=\"no-metric\">
        <summary>{{ name }}/</summary>
        {{ children|safe }}
      </details>
    {% endif %}
{% endmatch %}
"
)]
enum MetricTree {
    File {
        name: String,
        metric: String,
    },
    Folder {
        name: String,
        metric: Option<String>,
        children: MetricForest,
    },
}

#[derive(Template)]
#[template(
    ext = "html",
    source = "
<ul>
  {% for tree in trees %}
    <li>{{ tree|safe }}</li>
  {% endfor %}
</ul>
"
)]
struct MetricForest {
    trees: Vec<MetricTree>,
}

impl MetricForest {
    fn from_forest(children: HashMap<String, MetricFolder>) -> Self {
        let mut children = children.into_iter().collect::<Vec<_>>();
        children.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut trees = vec![];
        for (name, mut folder) in children {
            if let Some(file_metric) = folder.metric {
                trees.push(MetricTree::File {
                    name: name.clone(),
                    metric: file_metric,
                });
            }

            let is_folder = !folder.children.is_empty();
            let folder_metric = folder.children.remove("").and_then(|f| f.metric);
            if is_folder {
                trees.push(MetricTree::Folder {
                    name,
                    metric: folder_metric,
                    children: Self::from_forest(folder.children),
                })
            }
        }
        Self { trees }
    }
}

#[derive(Template)]
#[template(path = "pages/graph.html")]
struct Page {
    link_uplot_css: Link,
    link_graph_js: Link,
    base: Base,

    metrics: MetricForest,
}

pub async fn get_graph(
    _path: PathGraph,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let metrics =
        sqlx::query_scalar!("SELECT DISTINCT metric FROM run_measurements ORDER BY metric ASC")
            .fetch_all(&db)
            .await?;

    let metrics = MetricFolder::new(metrics);
    assert!(metrics.metric.is_none());
    let metrics = MetricForest::from_forest(metrics.children);

    let base = Base::new(config, Tab::Graph);
    Ok(Page {
        link_uplot_css: base.link(UPLOT_CSS),
        link_graph_js: base.link(GRAPH_JS),
        base,

        metrics,
    })
}

#[derive(Deserialize)]
pub struct QueryGraphData {
    #[serde(default)]
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

    // TODO Limit by date or amount

    let mut unsorted_hashes = Vec::<String>::new();
    let mut times_by_hash = HashMap::<String, i64>::new();
    let mut rows = sqlx::query!(
        "\
        SELECT \
            hash, \
            committer_date AS \"time: OffsetDateTime\" \
        FROM commits \
        WHERE reachable = 2 \
        ORDER BY hash ASC \
        "
    )
    .fetch(&mut *conn);
    while let Some(row) = rows.next().await {
        let row = row?;
        unsorted_hashes.push(row.hash.clone());
        times_by_hash.insert(row.hash, row.time.unix_timestamp());
    }
    drop(rows);

    let parent_child_pairs = sqlx::query!(
        "\
        SELECT parent, child \
        FROM commit_links \
        JOIN commits ON hash = parent \
        WHERE reachable = 2 \
        ORDER BY parent ASC, child ASC \
        "
    )
    .fetch(&mut *conn)
    .map_ok(|r| (r.parent, r.child))
    .try_collect::<Vec<_>>()
    .await?;

    let mut hashes = util::sort_topologically(&unsorted_hashes, &parent_child_pairs);
    hashes.sort_by_key(|hash| times_by_hash[hash]);

    let sorted_hash_indices = hashes
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, hash)| (hash, i))
        .collect::<HashMap<_, _>>();

    let mut parents = HashMap::<usize, Vec<usize>>::new();
    for (parent, child) in &parent_child_pairs {
        if let Some(parent_idx) = sorted_hash_indices.get(parent) {
            if let Some(child_idx) = sorted_hash_indices.get(child) {
                parents.entry(*parent_idx).or_default().push(*child_idx);
            }
        }
    }

    // Collect times
    let times = hashes
        .iter()
        .map(|hash| times_by_hash[hash])
        .collect::<Vec<_>>();

    // permutation[unsorted_index] = sorted_index
    let permutation = unsorted_hashes
        .iter()
        .map(|hash| sorted_hash_indices[hash])
        .collect::<Vec<_>>();

    // Collect and permutate measurements
    let mut measurements = HashMap::new();
    for metric in form.metric {
        let mut values = vec![None; hashes.len()];
        let mut rows = sqlx::query_scalar!(
            "\
            WITH \
            measurements AS ( \
                SELECT hash, value, MAX(start) \
                FROM runs \
                JOIN run_measurements USING (id) \
                WHERE metric = ? \
                GROUP BY hash \
            ) \
            SELECT value \
            FROM commits \
            LEFT JOIN measurements USING (hash) \
            WHERE reachable = 2 \
            ORDER BY hash ASC \
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
        hashes,
        parents,
        times,
        measurements,
    }))
}
