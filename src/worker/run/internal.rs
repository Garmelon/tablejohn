use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use regex::RegexBuilder;
use walkdir::WalkDir;

use crate::{
    shared::{Direction, Measurement},
    somehow,
    worker::server::Server,
};

use super::{Finished, RunInProgress};

#[derive(Default)]
struct Counts {
    files_by_ext: HashMap<String, usize>,
    lines_by_ext: HashMap<String, usize>,
    todos_by_ext: HashMap<String, usize>,
}

fn count(run: &RunInProgress, path: &Path) -> somehow::Result<Counts> {
    let todo_regex = RegexBuilder::new(r"[^a-z]todo[^a-z]")
        .case_insensitive(true)
        .build()
        .unwrap();

    let mut counts = Counts::default();
    for entry in WalkDir::new(path).sort_by_file_name() {
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

        let relative_path = entry
            .path()
            .components()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(entry.depth())
            .rev()
            .collect::<PathBuf>();
        run.log_stdout(format!(
            "{} has {lines} line{}, {todos} todo{}",
            relative_path.display(),
            if lines == 1 { "" } else { "s" },
            if todos == 1 { "" } else { "s" },
        ));
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

impl RunInProgress {
    pub(super) async fn perform_internal(
        &self,
        server: &Server,
    ) -> somehow::Result<Option<Finished>> {
        let run = self.clone();
        let dir = server.download_repo(&self.run.hash).await?;
        let path = dir.path().to_path_buf();
        let counts = tokio::task::spawn_blocking(move || count(&run, &path)).await??;
        Ok(Some(Finished {
            exit_code: 0,
            measurements: measurements(counts),
        }))
    }
}
