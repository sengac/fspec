//! REFAC-022: SessionManagerHandle trait split into composed fragments.
//!
//! Feature: spec/features/session-manager-handle-split.feature

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use std::path::{Path, PathBuf};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn handle_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/session_manager_handle.rs")
}

#[test]
fn every_session_manager_handle_source_file_stays_under_300_lines() -> TestResult {
    // @step Given rust/core/src/session_manager_handle.rs and its fragment directory
    let mut files = vec![handle_rs()];
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src/session_manager_handle")];
    while let Some(dir) = dirs.pop() {
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }

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

#[test]
fn the_composed_trait_keeps_its_methods() -> TestResult {
    // @step Given the composed source of rust/core/src/session_manager_handle.rs
    let path = handle_rs();

    // @step When it is read through source_composition::read_source
    let source = codelet_test_helpers::source_composition::read_source(&path)?;

    // @step Then it declares pub trait SessionManagerHandle with fn reconcile_work_units and fn loop_list
    assert!(source.contains("pub trait SessionManagerHandle: Send + Sync + 'static {"));
    assert!(source.contains("fn reconcile_work_units(&self"));
    assert!(source.contains("fn loop_list(&self, session_id: &SessionId)"));

    // @step And StubSessionManagerHandle still implements the trait for a dyn SessionManagerHandle
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(StubSessionManagerHandle::new());
    assert!(handle.list_sessions("")?.is_empty());
    Ok(())
}
