use axum::{extract::State, response::IntoResponse};
use maud::html;

use crate::{
    config::ServerConfig,
    server::web::{
        base::{Base, Tab},
        paths::PathTest,
    },
    somehow,
};

pub async fn get_test(
    _path: PathTest,
    State(config): State<&'static ServerConfig>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Index);

    Ok(base.html(
        "test",
        html! {},
        html! {
            h2 { "Test" }
            p { "Hello world!" }
        },
    ))
}
