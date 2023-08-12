mod internal;
mod repo;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::{debug_span, error, Instrument};

use crate::{
    id,
    shared::{BenchMethod, FinishedRun, Measurement, Source, UnfinishedRun},
};

const LIVE_SCROLLBACK: usize = 50;

pub enum FullRunStatus {
    Unfinished(UnfinishedRun),
    Finished(FinishedRun),
    Aborted,
}

#[derive(Clone)]
pub enum RunStatus {
    Unfinished,
    Finished {
        end: OffsetDateTime,
        exit_code: i32,
        measurements: HashMap<String, Measurement>,
    },
    Aborted,
}

#[derive(Clone)]
pub struct Run {
    id: String,
    hash: String,
    start: OffsetDateTime,
    output: Vec<(Source, String)>,
    status: RunStatus,
}

impl Run {
    pub fn new(id: String, hash: String) -> Self {
        Self {
            id,
            hash,
            start: OffsetDateTime::now_utc(),
            output: vec![],
            status: RunStatus::Unfinished,
        }
    }

    pub fn into_full_status(self) -> FullRunStatus {
        match self.status {
            RunStatus::Unfinished => FullRunStatus::Unfinished(UnfinishedRun {
                id: self.id,
                hash: self.hash,
                start: self.start,
                last_output: self
                    .output
                    .into_iter()
                    .rev()
                    .take(LIVE_SCROLLBACK)
                    .rev()
                    .collect(),
            }),

            RunStatus::Finished {
                end,
                exit_code,
                measurements,
            } => FullRunStatus::Finished(FinishedRun {
                id: self.id,
                hash: self.hash,
                start: self.start,
                end,
                exit_code,
                measurements,
                output: self.output,
            }),

            RunStatus::Aborted => FullRunStatus::Aborted,
        }
    }
}

pub async fn run(
    run: Arc<Mutex<Run>>,
    abort_rx: mpsc::UnboundedReceiver<()>,
    bench_method: BenchMethod,
) {
    async {
        let result = match bench_method {
            BenchMethod::Internal => internal::run(run, abort_rx).await,
            BenchMethod::Repo { hash } => repo::run(run, hash, abort_rx).await,
        };
        match result {
            Ok(()) => {}
            Err(e) => error!("Error during run:\n{e:?}"),
        }
    }
    .instrument(debug_span!("run"))
    .await;
}
