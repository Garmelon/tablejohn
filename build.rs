use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

fn watch_dir(path: &Path) {
    WalkDir::new(path)
        .into_iter()
        .for_each(|e| println!("cargo:rerun-if-changed={}", e.unwrap().path().display()));
}

fn run_tsc(static_out_dir: &Path) {
    let status = Command::new("tsc")
        .arg("--outDir")
        .arg(static_out_dir)
        .status()
        .unwrap();

    assert!(status.success(), "tsc produced errors");
}

fn copy_static_files(static_dir: &Path, static_out_dir: &Path) {
    let files = WalkDir::new(static_dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file());

    for file in files {
        let components = file.path().components().collect::<Vec<_>>();
        let relative_path = components.into_iter().rev().take(file.depth()).rev();
        let mut target = static_out_dir.to_path_buf();
        target.extend(relative_path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(file.path(), target).unwrap();
    }
}

fn main() {
    let mut builder = vergen::EmitBuilder::builder();
    builder.git_sha(false);
    builder.emit().unwrap();

    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let static_out_dir = out_dir.join("static");

    // Since remove_dir_all fails if the directory doesn't exist, we ensure it
    // exists before deleting it. This way, we can use the remove_dir_all Result
    // to ensure the directory was deleted successfully.
    fs::create_dir_all(&static_out_dir).unwrap();
    fs::remove_dir_all(&static_out_dir).unwrap();

    watch_dir("scripts".as_ref());
    run_tsc(&static_out_dir);

    watch_dir("static".as_ref());
    copy_static_files("static".as_ref(), &static_out_dir);
}
