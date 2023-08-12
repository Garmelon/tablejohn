use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::somehow;

use super::{Run, RunStatus};

pub async fn run(
    run: Arc<Mutex<Run>>,
    hash: String,
    abort_rx: mpsc::UnboundedReceiver<()>,
) -> somehow::Result<RunStatus> {
    todo!()
}
