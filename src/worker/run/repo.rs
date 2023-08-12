use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::somehow;

use super::Run;

pub async fn run(
    run: Arc<Mutex<Run>>,
    hash: String,
    abort_rx: mpsc::UnboundedReceiver<()>,
) -> somehow::Result<()> {
    todo!()
}
