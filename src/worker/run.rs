use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::shared::{Run, Source};

pub struct RunInProgress {
    pub server_name: String,
    pub run: Run,
    pub output: Arc<Mutex<Vec<(Source, String)>>>,
    pub abort: mpsc::UnboundedSender<()>,
}
