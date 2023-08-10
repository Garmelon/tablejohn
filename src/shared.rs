//! Data structures modelling the communication between server and runner.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::OffsetDateTime;

fn is_false(b: &bool) -> bool {
    !b
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stddev: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
}

#[derive(Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Source {
    // Stdin would be fd 0
    Stdout = 1,
    Stderr = 2,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UnfinishedRun {
    pub id: String,
    pub hash: String,
    pub start: OffsetDateTime,
    #[serde(default)]
    pub last_output: Vec<(Source, String)>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FinishedRun {
    pub id: String,
    pub hash: String,
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
    #[serde(default)]
    pub exit_code: i32,
    pub measurements: HashMap<String, Measurement>,
    #[serde(default)]
    pub output: Vec<(Source, String)>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum RunnerStatus {
    /// The runner is not performing any work.
    Idle,
    /// The runner is performing work for another server.
    Busy,
    /// The runner is performing work for the current server.
    Working(UnfinishedRun),
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub request_work: bool,

    /// The runner has finished a run and wants to submit the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_work: Option<FinishedRun>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum BenchMethod {
    /// Use internal (deterministic) benchmarking code.
    Internal,
    /// Use a commit from a bench repo.
    Repo { hash: String },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<Work>,

    /// The runner should abort the current run.
    ///
    /// The server may send this because it detected the runner is benchmarking
    /// the same commit as another runner and has broken the tie in favor of the
    /// other runner. The runner may continue the run despite this flag.
    #[serde(default, skip_serializing_if = "is_false")]
    pub abort_work: bool,
}
