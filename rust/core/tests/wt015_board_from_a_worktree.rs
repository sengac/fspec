//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! The codelet-core board writer (the TUI's reorder) and the board watcher
//! resolve the board through the state-root pointer. "A plain copy" is a
//! directory holding a stale copy of the main checkout's spec/ and a
//! `.fspec/state-root` pointer as fspec writes it; no git is involved.

use std::fs;
use std::path::{Path, PathBuf};

use codelet_core::work_units::{read_snapshot, WorkUnitsWatcher};
use codelet_core::work_units_write::{move_work_unit, Direction};
use codelet_test_helpers::{TestError, TestResult};
use serde_json::{json, Value};

fn board_json(ids: &[&str]) -> Value {
    let mut units = serde_json::Map::new();
    for id in ids {
        units.insert(
            (*id).to_string(),
            json!({"id": id, "title": id, "type": "story", "status": "specifying",
                   "createdAt": "2026-11-07T00:00:00.000Z",
                   "updatedAt": "2026-11-07T00:00:00.000Z"}),
        );
    }
    json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": units,
        "states": {
            "backlog": [], "specifying": ids, "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    })
}

fn write_board(root: &Path, ids: &[&str]) -> Result<(), TestError> {
    fs::create_dir_all(root.join("spec"))?;
    fs::write(root.join(".gitignore"), ".fspec/\n")?;
    fs::write(
        root.join("spec").join("work-units.json"),
        serde_json::to_string_pretty(&board_json(ids))?,
    )?;
    Ok(())
}

/// Main's board has STORY-001..003; the copy's stale board lacks STORY-003.
fn main_and_copy() -> Result<(tempfile::TempDir, PathBuf, PathBuf), TestError> {
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    write_board(&main, &["STORY-001", "STORY-002", "STORY-003"])?;
    let copy = main.join(".fspec").join("worktrees").join("s1");
    write_board(&copy, &["STORY-001", "STORY-002"])?;
    fs::create_dir_all(copy.join(".fspec"))?;
    fs::write(
        copy.join(".fspec").join("state-root"),
        json!({"stateRoot": main, "sessionId": "s1"}).to_string(),
    )?;
    Ok((dir, main, copy))
}

fn specifying(root: &Path) -> Result<Vec<String>, TestError> {
    let board: Value = serde_json::from_str(&fs::read_to_string(
        root.join("spec").join("work-units.json"),
    )?)?;
    Ok(board["states"]["specifying"]
        .as_array()
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

#[test]
fn the_board_writer_and_watcher_use_the_main_checkouts_board_from_a_worktree() -> TestResult {
    // @step Given a main checkout with a board and a plain copy of it holding a state-root pointer
    let (_dir, main, copy) = main_and_copy()?;
    let copy_before = fs::read_to_string(copy.join("spec").join("work-units.json"))?;

    // @step When the codelet-core board writer moves a card with the copy as its workspace
    move_work_unit(&copy, "STORY-003", Direction::Up)?;

    // @step Then the card moves on the main checkout's board
    let expected = vec!["STORY-001", "STORY-003", "STORY-002"];
    assert_eq!(specifying(&main)?, expected);
    assert_eq!(
        fs::read_to_string(copy.join("spec").join("work-units.json"))?,
        copy_before
    );

    // @step And the board watcher for the copy reads the main checkout's spec/work-units.json
    let snapshot: Vec<String> = read_snapshot(&copy)?.into_iter().map(|u| u.id).collect();
    assert_eq!(snapshot, expected);
    let watcher = WorkUnitsWatcher::new(&copy)?;
    let watched: Vec<String> = watcher.snapshot().into_iter().map(|u| u.id).collect();
    assert_eq!(watched, expected);
    Ok(())
}
