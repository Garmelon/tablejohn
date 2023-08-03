use axum::{routing::get, Router};

async fn run() -> anyhow::Result<()> {
    let app = Router::new().route("/", get(|| async { "Hello, world!" }));

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
