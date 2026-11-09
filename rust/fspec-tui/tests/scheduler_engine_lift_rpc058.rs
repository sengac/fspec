//! RPC-058 — Source-shape assertions for the scheduler-engine LIFT.
//!
//! Feature: spec/features/rpc058-scheduler-engine-lift.feature
//!
//! These tests pin the file layout for the lift of the scheduler
//! engine out of `rust/napi/src/scheduler/` and into
//! `rust/core/src/scheduler/`, plus the new `SchedulerHooks` trait
//! that replaces direct `crate::session_bindings::SessionManager`
//! references. The pure CRUD helpers from
//! `rust/napi/src/schedule_handler.rs` lift into
//! `rust/core/src/scheduler/crud.rs`.
//!
//! These tests are pure file scans — no compile dependency on the
//! lifted modules themselves — so they catch refactors that
//! accidentally re-introduce a NAPI dependency into the engine.

use codelet_test_helpers::OrFail;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .and_then(|p| p.parent())
        .or_fail("workspace root above rust/fspec-tui")?
        .to_path_buf())
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn scheduler_core_dir() -> Result<PathBuf, codelet_test_helpers::TestError> {
    Ok(workspace_root()?.join("rust/core/src/scheduler"))
}

/// Recursively read every regular file under `dir` and return a Vec
/// of (relative-path, contents).
fn read_all_rust_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Ok(content) = codelet_test_helpers::source_composition::read_tree_or_file(&path)
            {
                out.push((path, content));
            }
        }
    }
    out
}

#[path = "scheduler_engine_lift_rpc058/lifted_scheduler_engine_and.rs"]
mod lifted_scheduler_engine_and;
#[path = "scheduler_engine_lift_rpc058/scheduler_engine_modules_live.rs"]
mod scheduler_engine_modules_live;
