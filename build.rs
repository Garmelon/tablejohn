use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

const MIGRATION_DIR: &str = "migrations";
const STATIC_DIR: &str = "static";
const TEMPLATE_DIR: &str = "templates";

const STATIC_OUT_DIR: &str = "target/static";
const STATIC_IGNORE_EXT: &[&str] = &["ts"];

fn watch_dir(path: &Path) {
    WalkDir::new(path)
        .into_iter()
        .for_each(|e| println!("cargo:rerun-if-changed={}", e.unwrap().path().display()));
}

fn run_tsc() {
    let status = Command::new("tsc").status().unwrap();
    assert!(status.success(), "tsc produced errors");
}

fn copy_static_files() {
    let files = WalkDir::new(STATIC_DIR)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let extension = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            !STATIC_IGNORE_EXT.contains(&extension)
        });

    for file in files {
        let components = file.path().components().collect::<Vec<_>>();
        let relative_path = components.into_iter().rev().take(file.depth()).rev();
        let mut target = PathBuf::new().join(STATIC_OUT_DIR);
        target.extend(relative_path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(file.path(), target).unwrap();
    }
}

fn main() {
    let mut builder = vergen::EmitBuilder::builder();
    builder.git_sha(false);
    builder.emit().unwrap();

    watch_dir(MIGRATION_DIR.as_ref());
    watch_dir(STATIC_DIR.as_ref());
    watch_dir(TEMPLATE_DIR.as_ref());

    // Since remove_dir_all fails if the directory doesn't exist, we ensure it
    // exists before deleting it. This way, we can use the remove_dir_all Result
    // to ensure the directory was deleted successfully.
    fs::create_dir_all(STATIC_OUT_DIR).unwrap();
    fs::remove_dir_all(STATIC_OUT_DIR).unwrap();

    run_tsc();
    copy_static_files();
}
