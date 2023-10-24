use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::Hasher,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::{DirEntry, WalkDir};

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

fn relative_path(entry: &DirEntry) -> PathBuf {
    entry
        .path()
        .components()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .take(entry.depth())
        .rev()
        .collect::<PathBuf>()
}

fn copy_static_files(static_dir: &Path, static_out_dir: &Path) {
    let files = WalkDir::new(static_dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file());

    for file in files {
        let target = static_out_dir.to_path_buf().join(relative_path(&file));
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(file.path(), target).unwrap();
    }
}

fn make_static_constants(static_out_dir: &Path, static_out_file: &Path) {
    let files = WalkDir::new(static_out_dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file());

    let mut definitions = String::new();

    for file in files {
        let relative_path = relative_path(&file);
        let relative_path = relative_path.to_str().unwrap();

        let mut hasher = DefaultHasher::new();
        hasher.write(&fs::read(file.path()).unwrap());
        let hash = hasher.finish() & 0xffffffff;

        let name = relative_path
            .split(|c: char| !c.is_ascii_alphabetic())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_")
            .to_uppercase();
        let path = format!("/{relative_path}?h={hash:x}");
        definitions.push_str("#[allow(dead_code)]\n");
        definitions.push_str(&format!("pub const {name}: &str = {path:?};\n"));
    }

    fs::write(static_out_file, definitions).unwrap();
}

fn main() {
    let mut builder = vergen::EmitBuilder::builder();
    builder.git_sha(false);
    builder.emit().unwrap();

    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let static_out_dir = out_dir.join("static");
    let static_out_file = out_dir.join("static.rs");

    // Since remove_dir_all fails if the directory doesn't exist, we ensure it
    // exists before deleting it. This way, we can use the remove_dir_all Result
    // to ensure the directory was deleted successfully.
    fs::create_dir_all(&static_out_dir).unwrap();
    fs::remove_dir_all(&static_out_dir).unwrap();

    watch_dir("scripts".as_ref());
    run_tsc(&static_out_dir);

    watch_dir("static".as_ref());
    copy_static_files("static".as_ref(), &static_out_dir);

    make_static_constants(&static_out_dir, &static_out_file);
}
