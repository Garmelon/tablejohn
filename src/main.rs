mod r#static;

use askama::Template;
use axum::{routing::get, Router};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    greetee: String,
}

async fn run() -> anyhow::Result<()> {
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                IndexTemplate {
                    greetee: "world".to_string(),
                }
            }),
        )
        .fallback(get(r#static::static_handler));
    // TODO Add text body to body-less status codes

    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Rust-analyzer struggles analyzing code in this function, so the actual
    // code lives in a different function.
    run().await
}
