use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;
use tracing::error;

use crate::{
    shared::{BenchMethod, FinishedRun, Measurement, Run, Source},
    somehow,
};

struct Finished {
    exit_code: i32,
    measurements: HashMap<String, Measurement>,
}

// TODO Make fields private
#[derive(Clone)]
pub struct RunInProgress {
    pub server_name: String,
    pub run: Run,
    pub output: Arc<Mutex<Vec<(Source, String)>>>,
    pub abort: Arc<Notify>,
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

    pub fn log_stdout(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stdout, line));
    }

    pub fn log_stderr(&self, line: String) {
        self.output.lock().unwrap().push((Source::Stderr, line));
    }

    pub async fn perform(&self) -> Option<FinishedRun> {
        // TODO Remove type annotations
        // TODO Handle aborts
        let result: somehow::Result<_> = match &self.run.bench_method {
            BenchMethod::Internal => todo!(),
            BenchMethod::Repo { hash } => todo!(),
        };

        let finished = match result {
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
            run: self.run.clone(),
            exit_code: finished.exit_code,
            output,
            measurements: finished.measurements,
        })
    }
}
