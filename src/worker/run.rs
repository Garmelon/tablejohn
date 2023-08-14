mod internal;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;
use tracing::error;

use crate::{
    config::WorkerServerConfig,
    shared::{BenchMethod, FinishedRun, Measurement, Run, Source, UnfinishedRun},
    somehow,
};

use super::server::Server;

struct Finished {
    exit_code: i32,
    measurements: HashMap<String, Measurement>,
}

const SCROLLBACK: usize = 50;

#[derive(Clone)]
pub struct RunInProgress {
    server_name: String,
    server_config: &'static WorkerServerConfig,
    run: Run,
    output: Arc<Mutex<Vec<(Source, String)>>>,
    abort: Arc<Notify>,
}

impl RunInProgress {
    pub fn new(server_name: String, server_config: &'static WorkerServerConfig, run: Run) -> Self {
        Self {
            server_name,
            server_config,
            run,
            output: Arc::new(Mutex::new(vec![])),
            abort: Arc::new(Notify::new()),
        }
    }

    pub fn is_for_server(&self, name: &str) -> bool {
        self.server_name == name
    }

    pub fn as_unfinished_run(&self) -> UnfinishedRun {
        let last_output = self
            .output
            .lock()
            .unwrap()
            .iter()
            .rev()
            .take(SCROLLBACK)
            .rev()
            .cloned()
            .collect();
        UnfinishedRun {
            id: self.run.id.clone(),
            hash: self.run.hash.clone(),
            bench_method: self.run.bench_method.to_string(),
            start: self.run.start,
            last_output,
        }
    }

    pub fn log_stdout(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stdout, line));
    }

    pub fn log_stderr(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stderr, line));
    }

    pub async fn perform(&self, server: &Server) -> Option<FinishedRun> {
        // TODO Remove type annotations
        // TODO Handle aborts
        let result: somehow::Result<_> = match &self.run.bench_method {
            BenchMethod::Internal => self.perform_internal(server),
            BenchMethod::Repo { hash } => todo!(),
        }
        .await;

        let run = match result {
            Ok(outcome) => outcome,
            Err(e) => {
                error!("Error during run:\n{e:?}");
                self.log_stderr("Internal error:".to_string());
                self.log_stderr(format!("{e:?}"));
                Some(Finished {
                    exit_code: -1,
                    measurements: HashMap::new(),
                })
            }
        }?;

        let mut output = vec![];
        std::mem::swap(&mut output, &mut *self.output.lock().unwrap());

        Some(FinishedRun {
            id: self.run.id.clone(),
            hash: self.run.hash.clone(),
            bench_method: self.run.bench_method.to_string(),
            start: self.run.start,
            end: None,
            exit_code: run.exit_code,
            output,
            measurements: run.measurements,
        })
    }
}
