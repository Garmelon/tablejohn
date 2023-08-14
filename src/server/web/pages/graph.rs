use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{
    config::Config,
    server::web::{
        base::{Base, Link, Tab},
        paths::PathGraph,
        r#static::GRAPH_JS,
    },
    somehow,
};

#[derive(Template)]
#[template(path = "pages/graph.html")]
struct Page {
    link_graph_js: Link,
    base: Base,
}

pub async fn get_graph(
    _path: PathGraph,
    State(config): State<&'static Config>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Graph);
    Ok(Page {
        link_graph_js: base.link(GRAPH_JS),
        base,
    })
}
