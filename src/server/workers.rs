use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use crate::{
    config::ServerConfig,
    id,
    primitive::Timestamp,
    shared::{BenchMethod, Run, UnfinishedRun, WorkerStatus},
};

#[derive(Clone)]
pub struct WorkerInfo {
    pub secret: String,
    pub first_seen: Timestamp,
    pub last_seen: Timestamp,
    pub status: WorkerStatus,
}

impl WorkerInfo {
    pub fn new(secret: String, last_seen: Timestamp, status: WorkerStatus) -> Self {
        Self {
            secret,
            first_seen: Timestamp::now(),
            last_seen,
            status,
        }
    }
}

pub struct Workers {
    config: &'static ServerConfig,
    workers: HashMap<String, WorkerInfo>,
}

impl Workers {
    pub fn new(config: &'static ServerConfig) -> Self {
        Self {
            config,
            workers: HashMap::new(),
        }
    }

    pub fn clean(&mut self) -> &mut Self {
        let now = OffsetDateTime::now_utc();
        self.workers
            .retain(|_, v| now <= v.last_seen.0 + self.config.worker_timeout);
        self
    }

    pub fn verify_secret(&self, name: &str, secret: &str) -> bool {
        if let Some(worker) = self.workers.get(name) {
            worker.secret == secret
        } else {
            // The per-worker secret exists to prevent two workers from using
            // the same name at the same time (likely a misconfiguration). Since
            // we don't know a worker under this name yet, any secret is valid.
            true
        }
    }

    pub fn update(&mut self, name: String, info: WorkerInfo) {
        self.workers.insert(name, info);
    }

    pub fn find_and_reserve_run(
        &mut self,
        name: &str,
        queue: &[String],
        bench_method: BenchMethod,
    ) -> Option<Run> {
        let covered = self
            .workers
            .values()
            .filter_map(|info| match &info.status {
                WorkerStatus::Idle | WorkerStatus::Busy => None,
                WorkerStatus::Working(run) => Some(&run.hash),
            })
            .collect::<HashSet<_>>();

        // Find work not already covered by another worker
        let hash = queue.iter().find(|hash| !covered.contains(hash))?.clone();
        let id = id::random_run_id();
        let run = Run {
            id,
            hash,
            bench_method,
            start: Timestamp::now(),
        };

        // Reserve work so other workers don't choose it
        if let Some(info) = self.workers.get_mut(name) {
            info.status = WorkerStatus::Working(UnfinishedRun {
                id: run.id.clone(),
                hash: run.hash.clone(),
                bench_method: run.bench_method.to_string(),
                start: run.start,
                last_output: vec![],
            });
        }

        Some(run)
    }

    pub fn should_abort_work(&self, name: &str, queue: &[String]) -> bool {
        // A worker should abort work if...
        let Some(info) = self.workers.get(name) else {
            return false;
        };
        let WorkerStatus::Working(run) = &info.status else {
            return false;
        };

        // The commit isn't in the queue
        if !queue.contains(&run.hash) {
            return true;
        }

        // Another worker has been working on the same commit for longer
        let oldest_working_on_commit = self
            .workers
            .iter()
            .filter_map(|(name, info)| match &info.status {
                WorkerStatus::Working(u) if u.hash == run.hash => Some((name, u.start)),
                _ => None,
            })
            .max_by_key(|(_, start)| start.0)
            .map(|(name, _)| name as &str);
        if oldest_working_on_commit != Some(name) {
            return true;
        }

        false
    }

    pub fn get(&self, name: &str) -> Option<WorkerInfo> {
        self.workers.get(name).cloned()
    }

    pub fn get_all(&self) -> HashMap<String, WorkerInfo> {
        self.workers.clone()
    }
}
