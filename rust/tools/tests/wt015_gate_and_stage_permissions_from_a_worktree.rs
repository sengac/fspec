//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! The Fspec tool's RLCD gate state (built from the tool's project root,
//! the worktree of an isolated session since WT-016) and the stage
//! permissions (BLOCK-006) resolve shared state through the state-root
//! pointer. "A plain copy" is a directory holding a stale copy of the main
//! checkout's spec/ and a `.fspec/state-root` pointer as fspec writes it.
//! HOME is a per-process temp dir, so the user's own
//! ~/.fspec/stage-permissions.json is never read.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use codelet_test_helpers::{TestError, TestResult};
use codelet_tools::load_stage_permissions_config;
use codelet_tools::rlcd::build_gate_state;
use serde_json::{json, Value};

fn isolated_home() -> Result<(), TestError> {
    static HOME: OnceLock<Result<(), String>> = OnceLock::new();
    HOME.get_or_init(|| {
        let home = codelet_test_helpers::shared_temp_directory()?.join("home");
        fs::create_dir_all(&home).map_err(|e| e.to_string())?;
        std::env::set_var("HOME", &home);
        Ok(())
    })
    .clone()
    .map_err(Into::into)
}

fn board_json(ids: &[(&str, &str)]) -> Value {
    let mut units = serde_json::Map::new();
    for (id, title) in ids {
        units.insert(
            (*id).to_string(),
            json!({"id": id, "title": title, "type": "story", "status": "specifying",
                   "epic": "worktree-isolation"}),
        );
    }
    let specifying: Vec<&str> = ids.iter().map(|(id, _)| *id).collect();
    json!({"workUnits": units, "states": {"specifying": specifying}})
}

fn write_board(root: &Path, ids: &[(&str, &str)]) -> Result<(), TestError> {
    fs::create_dir_all(root.join("spec"))?;
    fs::write(root.join(".gitignore"), ".fspec/\n")?;
    fs::write(
        root.join("spec").join("work-units.json"),
        board_json(ids).to_string(),
    )?;
    Ok(())
}

/// Main has WT-900 and stage permissions; the copy's stale board lacks
/// WT-900 and the copy has no stage permissions of its own.
fn main_and_copy() -> Result<(tempfile::TempDir, PathBuf, PathBuf), TestError> {
    let dir = tempfile::tempdir()?;
    let main = dir.path().canonicalize()?.join("main");
    write_board(
        &main,
        &[
            ("WT-001", "Old card"),
            ("WT-900", "Card made after the copy"),
        ],
    )?;
    fs::create_dir_all(main.join(".fspec"))?;
    fs::write(
        main.join(".fspec").join("stage-permissions.json"),
        json!({
            "version": "1.0.0",
            "categories": [{"name": "wt015-main-only", "patterns": ["main-only/**"]}],
            "permissions": [{"stage": "testing", "writable_categories": ["wt015-main-only"]}]
        })
        .to_string(),
    )?;
    let copy = main.join(".fspec").join("worktrees").join("s1");
    write_board(&copy, &[("WT-001", "Old card")])?;
    fs::create_dir_all(copy.join(".fspec"))?;
    fs::write(
        copy.join(".fspec").join("state-root"),
        json!({"stateRoot": main, "sessionId": "s1"}).to_string(),
    )?;
    Ok((dir, main, copy))
}

#[test]
fn the_fspec_tool_gate_and_stage_permissions_in_a_worktree_read_the_main_checkouts_state(
) -> TestResult {
    isolated_home()?;
    // @step Given a main checkout with a board and stage permissions, and a plain copy of it holding a state-root pointer
    let (_dir, _main, copy) = main_and_copy()?;

    // @step When the Fspec tool's gate state is built and the stage permissions are loaded with the copy as the project root
    let gate = build_gate_state(
        "update-work-unit-status",
        &json!({"workUnitId": "WT-900", "status": "testing"}).to_string(),
        &copy.to_string_lossy(),
    );
    let permissions = load_stage_permissions_config(Some(&copy))?;

    // @step Then the gate state names the title of a card that exists only on the main checkout's board
    assert!(
        gate.contains("title=Card made after the copy"),
        "gate state: {gate}"
    );

    // @step And the main checkout's stage permissions are loaded
    assert!(
        permissions
            .categories
            .iter()
            .any(|category| category.name == "wt015-main-only"),
        "loaded categories: {:?}",
        permissions
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
    );
    Ok(())
}
