//! BUG-523 — fspec commands run from a subdirectory write the project's board.
//!
//! Feature: spec/features/commands-write-the-spec-directory-they-read.feature
//!
//! 'fspec update-work-unit-status BUG-481 implementing' ran with rust/ as
//! its working directory: it read and locked the repository's spec/ (found
//! by walking up) but wrote rust/spec/work-units.json, so the change was
//! lost. The CLI bridges now resolve the project root once, through
//! common::spec_project_root, before calling an fspec-core command.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

mod common;

use common::fspec_bin;

const SUBDIR: &str = "rust";

/// A project (marked by .gitignore) whose board has STAT-001, a task in
/// specifying, and a subdirectory "rust" with its own Cargo.toml.
fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    let write = |rel: &str, body: &str| {
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    };
    write(".gitignore", "");
    write("rust/Cargo.toml", "[workspace]\nmembers = []\n");
    write(
        "spec/work-units.json",
        &serde_json::to_string_pretty(&json!({
            "meta": {"version": "0.7.1", "lastUpdated": "2026-11-05T00:00:00.000Z"},
            "workUnits": {"STAT-001": {
                "id": "STAT-001", "title": "Status from a subdirectory", "type": "task",
                "status": "specifying",
                "createdAt": "2026-11-05T00:00:00.000Z", "updatedAt": "2026-11-05T00:00:00.000Z",
                "stateHistory": [{"state": "specifying", "timestamp": "2026-11-05T00:00:00.000Z"}]
            }},
            "states": {"backlog": [], "specifying": ["STAT-001"], "testing": [],
                       "implementing": [], "validating": [], "done": [], "blocked": []}
        }))
        .expect("serialize board"),
    );
    root
}

fn board(root: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(root.join("spec").join("work-units.json")).expect("read board"),
    )
    .expect("parse board")
}

/// Scenario: A status change run from a subdirectory lands in the project's board
#[test]
fn a_status_change_run_from_a_subdirectory_lands_in_the_project_board() {
    // @step Given a project whose board has the work unit "STAT-001" in specifying
    // @step And a subdirectory "rust" of the project with its own Cargo.toml
    let project = project();
    let root = project.path();

    // @step When "fspec update-work-unit-status STAT-001 blocked" runs with "rust" as its working directory
    let output = Command::new(fspec_bin())
        .args([
            "update-work-unit-status",
            "STAT-001",
            "blocked",
            "--blocked-reason",
            "waiting",
        ])
        .current_dir(root.join(SUBDIR))
        .output()
        .expect("spawn fspec");

    // @step Then it succeeds
    assert!(
        output.status.success(),
        "update-work-unit-status failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And "STAT-001" is blocked in the project's spec/work-units.json
    let board = board(root);
    assert_eq!(board["workUnits"]["STAT-001"]["status"], json!("blocked"));
    assert_eq!(board["states"]["blocked"], json!(["STAT-001"]));

    // @step And no spec directory exists in "rust"
    assert!(
        !root.join(SUBDIR).join("spec").exists(),
        "a stray rust/spec was created"
    );
}

/// The bridge modules: top-level files of the fspec crate's src/ that
/// call an fspec-core command (directly or through dispatch_command).
fn bridge_sources() -> Vec<(String, String)> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bridges = Vec::new();
    for entry in fs::read_dir(&src).expect("read src") {
        let path = entry.expect("dir entry").path();
        // common.rs holds the resolver itself (spec_project_root) and the
        // shared dispatch helpers; it is not a bridge.
        if path.extension().is_none_or(|e| e != "rs") || path.ends_with("common.rs") {
            continue;
        }
        let body = fs::read_to_string(&path).expect("read bridge");
        if body.contains("codelet_fspec_core::commands") || body.contains("dispatch_command") {
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .to_string();
            bridges.push((name, body));
        }
    }
    bridges.sort();
    bridges
}

/// init initialises the working directory it runs in (BUG-523 rule).
const INIT_BRIDGES: &[&str] = &["init.rs", "init_selector.rs"];

/// Scenario: Every CLI bridge takes its project root from the shared resolver
#[test]
fn every_cli_bridge_takes_its_project_root_from_the_shared_resolver() {
    // @step Given the fspec CLI bridge modules that call an fspec-core command
    // @step When their source is read
    let bridges = bridge_sources();
    assert!(
        bridges.len() > 150,
        "found only {} bridges; the scan is broken",
        bridges.len()
    );

    // @step Then every bridge except init and astgrep resolves its project root with common::spec_project_root
    let unresolved: Vec<&str> = bridges
        .iter()
        .filter(|(name, _)| !INIT_BRIDGES.contains(&name.as_str()))
        .filter(|(_, body)| !body.contains("spec_project_root()"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        unresolved.is_empty(),
        "bridges not using common::spec_project_root: {unresolved:?}"
    );
    assert!(
        bridges.iter().all(|(name, _)| name != "astgrep.rs"),
        "astgrep calls no fspec-core command and searches relative to the working directory"
    );

    // @step And no bridge passes the raw working directory to an fspec-core command
    let raw_cwd: Vec<&str> = bridges
        .iter()
        .filter(|(name, _)| !INIT_BRIDGES.contains(&name.as_str()))
        .filter(|(_, body)| body.contains("current_dir()"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        raw_cwd.is_empty(),
        "bridges reading the working directory directly: {raw_cwd:?}"
    );
}
