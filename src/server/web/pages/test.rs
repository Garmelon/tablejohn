use axum::{extract::State, response::IntoResponse};
use maud::html;

use crate::{
    config::ServerConfig,
    server::web::{
        page::{Page, Tab},
        paths::PathTest,
    },
    somehow,
};

pub async fn get_test(
    _path: PathTest,
    State(config): State<&'static ServerConfig>,
) -> somehow::Result<impl IntoResponse> {
    let html = Page::new(config)
        .title("test")
        .tab(Tab::Index)
        .body(html! {
            h2 { "Test" }
            p { "Hello world!" }
        })
        .build();

    Ok(html)
}
