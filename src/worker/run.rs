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
    config::WorkerServerConfig,
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

impl RunStatus {
    pub fn finished(exit_code: i32, measurements: HashMap<String, Measurement>) -> Self {
        Self::Finished {
            end: OffsetDateTime::now_utc(),
            exit_code,
            measurements,
        }
    }
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

    pub fn log_stdout(&mut self, line: String) {
        self.output.push((Source::Stdout, line));
    }

    pub fn log_stderr(&mut self, line: String) {
        self.output.push((Source::Stderr, line));
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
    server_config: &'static WorkerServerConfig,
    poke_tx: mpsc::UnboundedSender<()>,
    run: Arc<Mutex<Run>>,
    bench: BenchMethod,
    abort_rx: mpsc::UnboundedReceiver<()>,
) {
    async {
        let result = match bench {
            BenchMethod::Internal => internal::run(server_config, run.clone(), abort_rx).await,
            BenchMethod::Repo { hash } => repo::run(run.clone(), hash, abort_rx).await,
        };
        match result {
            Ok(status) => {
                assert!(!matches!(status, RunStatus::Unfinished));
                run.lock().unwrap().status = status;
            }
            Err(e) => {
                error!("Error during run:\n{e:?}");
                let mut guard = run.lock().unwrap();
                guard.log_stderr("Internal error:".to_string());
                guard.log_stderr(format!("{e:?}"));
                guard.status = RunStatus::finished(-1, HashMap::new());
            }
        }
        let _ = poke_tx.send(());
    }
    .instrument(debug_span!("run"))
    .await;
}
