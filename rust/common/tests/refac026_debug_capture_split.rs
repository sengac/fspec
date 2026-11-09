//! REFAC-026: split oversized files under 300 lines.
//!
//! Feature: spec/features/debug-capture-split.feature

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn rust_files(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            rust_files(&entry?.path(), out)?;
        }
    } else if path.extension().is_some_and(|ext| ext == "rs") {
        out.push(path.to_path_buf());
    }
    Ok(())
}

#[test]
fn every_split_source_file_stays_under_300_lines() -> TestResult {
    // @step Given rust/common/src/debug_capture/mod.rs and their new module files
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&root.join("debug_capture"), &mut files)?;

    // @step When the line count of every Rust source file is measured
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(oversized.is_empty(), "oversized files: {oversized:?}");
    Ok(())
}
