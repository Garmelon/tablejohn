//! Data structures modelling the communication between server and runner.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::OffsetDateTime;

#[derive(Clone, Serialize_repr, Deserialize_repr)]
#[repr(i8)]
pub enum Direction {
    LessIsBetter = -1,
    Neutral = 0,
    MoreIsBetter = 1,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub value: f64,
    pub stddev: Option<f64>,
    pub unit: Option<String>,
    pub direction: Option<i8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
#[serde(tag = "type")]
pub enum Line {
    Stdout(String),
    Stderr(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FinishedRun {
    pub id: String,
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    pub output: Vec<Line>,
    pub exit_code: i32,
    pub measurements: HashMap<String, Measurement>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
#[serde(tag = "type")]
pub enum RunnerStatus {
    /// The runner is not performing any work.
    Idle,
    /// The runner is performing work for another server.
    Busy,
    /// The runner is performing work for the current server.
    Working {
        id: String,
        hash: String,
        since: OffsetDateTime,
        last_lines: Vec<Line>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RunnerRequest {
    /// Additional free-form info about the runner.
    ///
    /// This could for example be used to describe the runner's system specs.
    pub info: Option<String>,

    /// Secret for preventing name collisions.
    pub secret: String,

    /// What the runner is currently working on.
    pub status: RunnerStatus,

    /// Whether the runner wants new work from the server.
    ///
    /// If the server has a commit available, it should respond with a non-null
    /// [`Response::work`].
    pub request_work: bool,

    /// The runner has finished a run and wants to submit the results.
    pub submit_work: Option<FinishedRun>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
#[serde(tag = "type")]
pub enum BenchMethod {
    /// Use internal (deterministic) benchmarking code.
    Internal,
    /// Use a commit from a bench repo.
    BenchRepo { hash: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Work {
    /// Hash of commit to benchmark.
    pub hash: String,
    /// How to benchmark the commit.
    pub bench: BenchMethod,
}

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
    /// Work the runner requested using [`Request::request_work].
    ///
    /// The runner may ignore this work and do something else. However, until
    /// the next update request sent by the runner, the server will consider the
    /// runner as preparing to work on the commit, and will not give out the
    /// same commit to other runners.
    pub work: Option<Work>,

    /// The runner should abort the current run.
    ///
    /// The server may send this because it detected the runner is benchmarking
    /// the same commit as another runner and has broken the tie in favor of the
    /// other runner. The runner may continue the run despite this flag.
    pub abort_work: bool,
}
