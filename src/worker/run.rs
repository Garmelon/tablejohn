mod internal;
mod repo;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::{error, warn};
use tokio::{select, sync::Notify};

use crate::{
    primitive::Source,
    shared::{BenchMethod, FinishedRun, Measurement, Run, UnfinishedRun},
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
    run: Run,
    output: Arc<Mutex<Vec<(Source, String)>>>,
    abort: Arc<Notify>,
}

impl RunInProgress {
    pub fn new(server_name: String, run: Run) -> Self {
        Self {
            server_name,
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

    pub fn log_internal(&self, line: String) {
        self.output.lock().unwrap().push((Source::Internal, line));
    }

    pub fn log_stdout(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stdout, line));
    }

    pub fn log_stderr(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stderr, line));
    }

    async fn execute_bench_method(&self, server: &Server) -> somehow::Result<Option<Finished>> {
        match &self.run.bench_method {
            BenchMethod::Internal => self.execute_internal(server).await,
            BenchMethod::Repo { hash } => self.execute_repo(server, hash).await,
        }
    }

    pub async fn perform(&self, server: &Server) -> Option<FinishedRun> {
        // TODO Log system info

        let result = select! {
            result = self.execute_bench_method(server) => result,
            _ = self.abort.notified() => {
                warn!("Run for {} was aborted", server.name);
                Ok(None)
            },
        };

        let run = match result {
            Ok(outcome) => outcome,
            Err(e) => {
                error!("Error during run for {}:\n{e:?}", server.name);
                self.log_internal("Internal error:".to_string());
                self.log_internal(format!("{e:?}"));
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

    pub fn abort(&self) {
        self.abort.notify_one();
    }
}
