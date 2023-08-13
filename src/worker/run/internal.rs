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
    files: usize,
    lines: usize,
    todos: usize,
    files_by_ext: HashMap<String, usize>,
    lines_by_ext: HashMap<String, usize>,
    todos_by_ext: HashMap<String, usize>,
    files_by_dir: HashMap<String, usize>,
    lines_by_dir: HashMap<String, usize>,
    todos_by_dir: HashMap<String, usize>,
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

        let relative_path = entry
            .path()
            .components()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(entry.depth())
            .rev()
            .collect::<PathBuf>();

        let ext = entry
            .path()
            .extension()
            .unwrap_or("none".as_ref())
            .to_string_lossy()
            .to_string();

        let dir = relative_path
            .components()
            .next()
            .filter(|_| relative_path.components().count() > 1)
            .map(|c| c.as_os_str())
            .unwrap_or("none".as_ref())
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

        counts.files += 1;
        counts.lines += lines;
        counts.todos += todos;
        *counts.files_by_ext.entry(ext.clone()).or_default() += 1;
        *counts.lines_by_ext.entry(ext.clone()).or_default() += lines;
        *counts.todos_by_ext.entry(ext.clone()).or_default() += todos;
        *counts.files_by_dir.entry(dir.clone()).or_default() += 1;
        *counts.lines_by_dir.entry(dir.clone()).or_default() += lines;
        *counts.todos_by_dir.entry(dir.clone()).or_default() += todos;

        run.log_stdout(format!(
            "{} has {lines} line{}, {todos} todo{}",
            relative_path.display(),
            if lines == 1 { "" } else { "s" },
            if todos == 1 { "" } else { "s" },
        ));
    }

    Ok(counts)
}

fn measurement(value: f64, direction: Direction) -> Measurement {
    Measurement {
        value,
        stddev: None,
        unit: None,
        direction: Some(direction),
    }
}

fn measurements(counts: Counts) -> HashMap<String, Measurement> {
    let mut measurements = HashMap::new();

    // Files
    measurements.insert(
        "files".to_string(),
        measurement(counts.files as f64, Direction::Neutral),
    );
    for (ext, count) in counts.files_by_ext {
        measurements.insert(
            format!("files/by ext/{ext}"),
            measurement(count as f64, Direction::Neutral),
        );
    }
    for (dir, count) in counts.files_by_dir {
        measurements.insert(
            format!("files/by dir/{dir}"),
            measurement(count as f64, Direction::Neutral),
        );
    }

    // Lines
    measurements.insert(
        "lines".to_string(),
        measurement(counts.lines as f64, Direction::Neutral),
    );
    for (ext, count) in counts.lines_by_ext {
        measurements.insert(
            format!("lines/by ext/{ext}"),
            measurement(count as f64, Direction::Neutral),
        );
    }
    for (dir, count) in counts.lines_by_dir {
        measurements.insert(
            format!("lines/by dir/{dir}"),
            measurement(count as f64, Direction::Neutral),
        );
    }

    // Todos
    measurements.insert(
        "todos".to_string(),
        measurement(counts.todos as f64, Direction::LessIsBetter),
    );
    for (ext, count) in counts.todos_by_ext {
        measurements.insert(
            format!("todos/by ext/{ext}"),
            measurement(count as f64, Direction::LessIsBetter),
        );
    }
    for (dir, count) in counts.todos_by_dir {
        measurements.insert(
            format!("todos/by dir/{dir}"),
            measurement(count as f64, Direction::LessIsBetter),
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
