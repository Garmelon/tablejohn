use askama::Template;
use axum::{extract::State, response::IntoResponse};
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    number: i32,
}

pub async fn get(State(db): State<SqlitePool>) -> super::Result<impl IntoResponse> {
    let result = sqlx::query!("SELECT column1 AS number FROM (VALUES (1))")
        .fetch_one(&db)
        .await?;

    let number = result.number;
    Ok(IndexTemplate { number })
}
