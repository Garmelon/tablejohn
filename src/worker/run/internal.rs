use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::{Arc, Mutex},
};

use regex::RegexBuilder;
use tokio::{select, sync::mpsc};
use tracing::debug;
use walkdir::WalkDir;

use crate::{
    config::WorkerServerConfig,
    shared::{Direction, Measurement},
    somehow,
    worker::{run::RunStatus, tree::UnpackedTree},
};

use super::Run;

#[derive(Default)]
struct Counts {
    files_by_ext: HashMap<String, usize>,
    lines_by_ext: HashMap<String, usize>,
    todos_by_ext: HashMap<String, usize>,
}

fn count(path: &Path) -> somehow::Result<Counts> {
    let todo_regex = RegexBuilder::new(r"[^a-z]todo[^a-z]")
        .case_insensitive(true)
        .build()
        .unwrap();

    let mut counts = Counts::default();
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let extension = entry
            .path()
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut lines = 0;
        let mut todos = 0;
        for line in BufReader::new(File::open(entry.path())?).lines() {
            let line = line?;
            lines += 1;
            if todo_regex.is_match(&line) {
                todos += 1;
            }
        }

        *counts.files_by_ext.entry(extension.clone()).or_default() += 1;
        *counts.lines_by_ext.entry(extension.clone()).or_default() += lines;
        *counts.todos_by_ext.entry(extension.clone()).or_default() += todos;
    }

    Ok(counts)
}

fn measurements(counts: Counts) -> HashMap<String, Measurement> {
    let mut measurements = HashMap::new();

    // Files
    measurements.insert(
        "files".to_string(),
        Measurement {
            value: counts.files_by_ext.values().sum::<usize>() as f64,
            stddev: None,
            unit: None,
            direction: Some(Direction::Neutral),
        },
    );
    for (extension, count) in counts.files_by_ext {
        measurements.insert(
            format!("files.{extension}"),
            Measurement {
                value: count as f64,
                stddev: None,
                unit: None,
                direction: Some(Direction::Neutral),
            },
        );
    }

    // Lines
    measurements.insert(
        "lines".to_string(),
        Measurement {
            value: counts.lines_by_ext.values().sum::<usize>() as f64,
            stddev: None,
            unit: None,
            direction: Some(Direction::Neutral),
        },
    );
    for (extension, count) in counts.lines_by_ext {
        measurements.insert(
            format!("lines.{extension}"),
            Measurement {
                value: count as f64,
                stddev: None,
                unit: None,
                direction: Some(Direction::Neutral),
            },
        );
    }

    // Todos
    measurements.insert(
        "todos".to_string(),
        Measurement {
            value: counts.todos_by_ext.values().sum::<usize>() as f64,
            stddev: None,
            unit: None,
            direction: Some(Direction::LessIsBetter),
        },
    );
    for (extension, count) in counts.todos_by_ext {
        measurements.insert(
            format!("todos.{extension}"),
            Measurement {
                value: count as f64,
                stddev: None,
                unit: None,
                direction: Some(Direction::LessIsBetter),
            },
        );
    }

    measurements
}

pub async fn run(
    server_config: &'static WorkerServerConfig,
    run: Arc<Mutex<Run>>,
    mut abort_rx: mpsc::UnboundedReceiver<()>,
) -> somehow::Result<RunStatus> {
    let hash = run.lock().unwrap().hash.clone();
    let url = format!("{}api/worker/repo/{}", server_config.url, hash);
    let tree = select! {
        r = UnpackedTree::download(&url, hash) => Some(r?),
        _ = abort_rx.recv() => None,
    };
    let Some(tree) = tree else {
        debug!("Run aborted while downloading commit");
        return Ok(RunStatus::Aborted);
    };

    let path = tree.dir.path().to_path_buf();
    let counts = tokio::task::spawn_blocking(move || count(&path)).await??;

    Ok(RunStatus::finished(0, measurements(counts)))
}
