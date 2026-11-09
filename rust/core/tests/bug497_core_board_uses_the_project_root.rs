//! BUG-497 — the core board uses the resolved project root.
//!
//! Feature: spec/features/project-settings-and-the-core-board-use-the-resolved-project-root.feature

use std::path::PathBuf;

use codelet_core::work_units::read_snapshot;
use codelet_core::work_units_write::{move_work_unit, Direction};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A project root (boundary marker + spec/work-units.json with backlog
/// [A-1, A-2]) and its `rust/sub` subdirectory.
fn project() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("Cargo.toml"), "[workspace]\n")?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    let unit = |id: &str| json!({"id": id, "title": id, "type": "story", "status": "backlog"});
    let board = json!({
        "workUnits": {"A-1": unit("A-1"), "A-2": unit("A-2")},
        "states": {"backlog": ["A-1", "A-2"], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []},
    });
    std::fs::write(
        root.path().join("spec/work-units.json"),
        serde_json::to_string_pretty(&board)?,
    )?;
    let sub = root.path().join("rust/sub");
    std::fs::create_dir_all(&sub)?;
    Ok((root, sub))
}

#[test]
fn reordering_a_work_unit_from_a_subdirectory_updates_the_projects_board() -> TestResult {
    // @step Given a project whose backlog holds two work units
    let (root, sub) = project()?;
    // @step When the second work unit is moved up from a subdirectory
    move_work_unit(&sub, "A-2", Direction::Up)?;
    // @step Then the project's spec/work-units.json lists it first in backlog
    let board: Value = serde_json::from_str(&std::fs::read_to_string(
        root.path().join("spec/work-units.json"),
    )?)?;
    assert_eq!(board["states"]["backlog"], json!(["A-2", "A-1"]), "{board}");
    // @step And the subdirectory has no spec folder
    assert!(!sub.join("spec").exists());
    Ok(())
}

#[test]
fn the_board_snapshot_from_a_subdirectory_reads_the_projects_board() -> TestResult {
    // @step Given a project whose board has a work unit
    let (_root, sub) = project()?;
    // @step When the board snapshot is read from a subdirectory
    let snapshot = read_snapshot(&sub)?;
    // @step Then the snapshot contains that work unit
    assert!(snapshot.iter().any(|unit| unit.id == "A-1"), "{snapshot:?}");
    Ok(())
}
