mod recurring;
mod web;

use tokio::select;

use crate::{somehow, state::AppState};

pub async fn run(state: AppState) -> somehow::Result<()> {
    select! {
        e = web::run(state.clone()) => e,
        () = recurring::run(state.clone()) => Ok(()),
    }
}
