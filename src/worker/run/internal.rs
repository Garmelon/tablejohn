use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use regex::RegexBuilder;
use walkdir::WalkDir;

use crate::{shared::Measurement, somehow, worker::server::Server};

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

fn count(path: &Path) -> somehow::Result<Counts> {
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

        let mut lines = 0;
        let mut todos = 0;
        for line in BufReader::new(File::open(entry.path())?).lines() {
            match line {
                Ok(line) => {
                    lines += 1;
                    if todo_regex.is_match(&line) {
                        todos += 1;
                    }
                }
                Err(_) => {
                    // Probably a binary file
                    lines = 0;
                    todos = 0;
                }
            }
        }

        counts.files += 1;
        counts.lines += lines;
        counts.todos += todos;

        *counts.files_by_ext.entry(ext.clone()).or_default() += 1;
        *counts.lines_by_ext.entry(ext.clone()).or_default() += lines;
        *counts.todos_by_ext.entry(ext.clone()).or_default() += todos;

        for ancestor in relative_path.ancestors().skip(1) {
            let ancestor = ancestor.to_string_lossy();
            if ancestor.is_empty() {
                continue;
            }
            *counts.files_by_dir.entry(ancestor.to_string()).or_default() += 1;
            *counts.lines_by_dir.entry(ancestor.to_string()).or_default() += lines;
            *counts.todos_by_dir.entry(ancestor.to_string()).or_default() += todos;
        }
    }

    // Avoid excessive amounts of data in very large repos
    if counts.files_by_dir.len() > 1000 {
        counts.files_by_dir.retain(|name, _| !name.contains('/'));
    }
    if counts.lines_by_dir.len() > 1000 {
        counts.lines_by_dir.retain(|name, _| !name.contains('/'));
    }
    if counts.todos_by_dir.len() > 1000 {
        counts.todos_by_dir.retain(|name, _| !name.contains('/'));
    }

    Ok(counts)
}

fn measurement(value: f64) -> Measurement {
    Measurement { value, unit: None }
}

fn measurements(counts: Counts) -> HashMap<String, Measurement> {
    let mut measurements = HashMap::new();

    // Files
    measurements.insert("files".to_string(), measurement(counts.files as f64));
    for (ext, count) in counts.files_by_ext {
        measurements.insert(
            format!("files/by extension/{ext}"),
            measurement(count as f64),
        );
    }
    for (dir, count) in counts.files_by_dir {
        measurements.insert(format!("files/by dir/{dir}/"), measurement(count as f64));
    }

    // Lines
    measurements.insert("lines".to_string(), measurement(counts.lines as f64));
    for (ext, count) in counts.lines_by_ext {
        measurements.insert(
            format!("lines/by extension/{ext}"),
            measurement(count as f64),
        );
    }
    for (dir, count) in counts.lines_by_dir {
        measurements.insert(format!("lines/by dir/{dir}/"), measurement(count as f64));
    }

    // Todos
    measurements.insert("todos".to_string(), measurement(counts.todos as f64));
    for (ext, count) in counts.todos_by_ext {
        measurements.insert(
            format!("todos/by extension/{ext}"),
            measurement(count as f64),
        );
    }
    for (dir, count) in counts.todos_by_dir {
        measurements.insert(format!("todos/by dir/{dir}/"), measurement(count as f64));
    }

    measurements
}

impl RunInProgress {
    pub(super) async fn perform_internal(
        &self,
        server: &Server,
    ) -> somehow::Result<Option<Finished>> {
        let dir = server.download_repo(&self.run.hash).await?;
        let path = dir.path().to_path_buf();
        let counts = tokio::task::spawn_blocking(move || count(&path)).await??;
        Ok(Some(Finished {
            exit_code: 0,
            measurements: measurements(counts),
        }))
    }
}
