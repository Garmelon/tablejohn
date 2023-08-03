use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const STATIC_DIR: &str = "static";
const STATIC_OUT_DIR: &str = "target/static";

fn copy_recursively(path: &Path) {
    let from = PathBuf::new().join(STATIC_DIR).join(path);
    let to = PathBuf::new().join(STATIC_OUT_DIR).join(path);

    println!("cargo:rerun-if-changed={}", from.display());

    if from.is_file() {
        if from.extension() == Some("ts".as_ref()) {
            return;
        }
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(from, to).unwrap();
    } else if from.is_dir() {
        for entry in from.read_dir().unwrap() {
            copy_recursively(&path.join(entry.unwrap().file_name()));
        }
    } else {
        panic!("Unexpected file type at {}", from.display());
    }
}

fn main() {
    // Since remove_dir_all fails if the directory doesn't exist, we ensure it
    // exists before deleting it. This way, we can use the remove_dir_all Result
    // to ensure the directory was deleted successfully.
    fs::create_dir_all(STATIC_OUT_DIR).unwrap();
    fs::remove_dir_all(STATIC_OUT_DIR).unwrap();

    // Run typescript compiler
    let status = Command::new("tsc").status().unwrap();
    assert!(status.success(), "tsc produced errors");

    // Copy remaining static files
    copy_recursively("".as_ref());
}
