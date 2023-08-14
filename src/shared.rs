//! Data structures modelling the communication between server and worker.

use std::{collections::HashMap, fmt};

use serde::{de, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Clone, Copy)]
pub struct Rfc3339Time(pub OffsetDateTime);

impl serde::Serialize for Rfc3339Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.format(&Rfc3339).unwrap().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Rfc3339Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input: &str = serde::Deserialize::deserialize(deserializer)?;
        OffsetDateTime::parse(input, &Rfc3339)
            .map_err(de::Error::custom)
            .map(Self)
    }
}

#[derive(Clone, Serialize_repr, Deserialize_repr, sqlx::Type)]
#[repr(u8)]
pub enum Source {
    // TODO Internal = 0 ?
    // Stdin would be fd 0
    Stdout = 1,
    Stderr = 2,
}

#[derive(Clone, Serialize_repr, Deserialize_repr, sqlx::Type)]
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum BenchMethod {
    Internal,
    Repo { hash: String },
}

impl fmt::Display for BenchMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchMethod::Internal => write!(f, "internal"),
            BenchMethod::Repo { hash } => write!(f, "bench repo, hash {hash}"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub hash: String,
    pub bench_method: BenchMethod,
    pub start: Rfc3339Time,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UnfinishedRun {
    pub id: String,
    pub hash: String,
    pub bench_method: String,
    pub start: Rfc3339Time,

    #[serde(default)]
    pub last_output: Vec<(Source, String)>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FinishedRun {
    pub id: String,
    pub hash: String,
    pub bench_method: String,
    pub start: Rfc3339Time,

    /// Override the server's end time.
    ///
    /// Should not be used in normal operation, but can be used when importing
    /// completed runs from other sources.
    pub end: Option<Rfc3339Time>,

    #[serde(default)]
    pub exit_code: i32,

    #[serde(default)]
    pub output: Vec<(Source, String)>,

    #[serde(default)]
    pub measurements: HashMap<String, Measurement>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum WorkerStatus {
    /// The worker is not performing any work.
    Idle,
    /// The worker is performing work for another server.
    Busy,
    /// The worker is performing work for the current server.
    Working(UnfinishedRun),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    /// Additional free-form info about the worker.
    ///
    /// This could for example be used to describe the worker's system specs.
    pub info: Option<String>,

    /// Secret for preventing name collisions.
    pub secret: String,

    /// What the worker is currently working on.
    pub status: WorkerStatus,

    /// The worker wants a new run from the server.
    ///
    /// If the server has a commit available, it should respond with a non-null
    /// [`ServerResponse::run`].
    #[serde(default, skip_serializing_if = "is_false")]
    pub request_run: bool,

    /// The worker has finished a run and wants to submit the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_run: Option<FinishedRun>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
    /// Run the worker requested using [`WorkerRequest::request_run`].
    ///
    /// The worker may ignore this run and do something else. However, until the
    /// next update request sent by the worker, the server will consider the
    /// worker as preparing to work on the commit, and will not give out the
    /// same commit to other workers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,

    /// The worker should abort the current run.
    ///
    /// The server may send this because it detected the worker is benchmarking
    /// the same commit as another worker and has broken the tie in favor of the
    /// other worker. The worker may continue the run despite this flag.
    #[serde(default, skip_serializing_if = "is_false")]
    pub abort_run: bool,
}
