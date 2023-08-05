mod commit;
mod index;
mod r#static;

use axum::{routing::get, Router, Server};

use crate::{somehow, state::AppState};

pub async fn run(state: AppState) -> somehow::Result<()> {
    // TODO Add text body to body-less status codes

    let app = Router::new()
        .route("/", get(index::get))
        .route("/commit/", get(commit::get))
        .fallback(get(r#static::static_handler))
        .with_state(state.clone());

    Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
