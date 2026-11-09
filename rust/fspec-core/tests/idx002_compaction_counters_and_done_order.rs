//! IDX-002 — compaction resets the architecture-note counter, and the
//! done-time auto-compact keeps the done column's order.
//!
//! Feature: spec/features/stable-indices-critical-bug-fixes.feature
//!
//! `compact-work-unit` drops soft-deleted items, renumbers the survivors
//! from 0 and sets `nextNoteId` to the survivor count, so the next
//! `add-architecture-note` cannot reuse a live id. Moving a work unit to
//! done compacts it in memory and writes the board once: the done column
//! keeps its order and the new unit is appended (product owner,
//! 2026-11-06: append order, H-DONE-ORDER A).

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const STATES: [&str; 7] = [
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

fn dispatch(dir: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    })
}

fn succeeded(what: &str, result: &DispatchResult) -> TestResult {
    if !result.success {
        return Err(format!("{what} failed: {result:?}").into());
    }
    Ok(())
}

/// Write a board holding `units` (id -> work unit) with `columns`
/// (state -> ids); every other column is empty.
fn write_board(dir: &Path, units: Value, columns: &[(&str, Vec<&str>)]) -> TestResult {
    let mut states = serde_json::Map::new();
    for state in STATES {
        let ids = columns
            .iter()
            .find(|(name, _)| *name == state)
            .map(|(_, ids)| ids.clone())
            .unwrap_or_default();
        states.insert(state.to_string(), json!(ids));
    }
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": units,
            "states": states,
        }))?,
    )?;
    Ok(())
}

fn read_board(dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join("spec/work-units.json"),
    )?)?)
}

fn unit(id: &str, status: &str, extra: Value) -> Value {
    let mut unit = json!({
        "id": id, "title": format!("Unit {id}"), "type": "story", "status": status,
        "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
    });
    if let (Some(target), Some(fields)) = (unit.as_object_mut(), extra.as_object()) {
        for (key, value) in fields {
            target.insert(key.clone(), value.clone());
        }
    }
    unit
}

fn note(id: u64, text: &str, deleted: bool) -> Value {
    json!({"id": id, "text": text, "deleted": deleted, "createdAt": "2026-11-01T00:00:00.000Z"})
}

/// The ids of AUTH-001's architecture notes, in file order.
fn note_ids(board: &Value) -> Vec<u64> {
    board["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["id"].as_u64())
        .collect()
}

#[test]
fn prevent_id_collision_after_compaction() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given a work unit AUTH-001 has 5 architecture notes
    // @step And notes at indices 1 and 3 are soft-deleted
    let notes = json!([
        note(0, "Note A", false),
        note(1, "Note B", true),
        note(2, "Note C", false),
        note(3, "Note D", true),
        note(4, "Note E", false),
    ]);
    write_board(
        dir,
        json!({"AUTH-001": unit("AUTH-001", "done", json!({"architectureNotes": notes, "nextNoteId": 5}))}),
        &[("done", vec!["AUTH-001"])],
    )?;

    // @step When compact-work-unit removes deleted items and renumbers remaining notes
    let result = dispatch(dir, "compact-work-unit", json!({"workUnitId": "AUTH-001"}));
    succeeded("compact-work-unit", &result)?;
    let board = read_board(dir)?;
    assert_eq!(note_ids(&board), vec![0, 1, 2]);
    let texts: Vec<&str> = board["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["text"].as_str())
        .collect();
    assert_eq!(texts, vec!["Note A", "Note C", "Note E"]);

    // @step And compact-work-unit sets nextNoteId = 3 (correct field)
    assert_eq!(board["workUnits"]["AUTH-001"]["nextNoteId"], json!(3));
    assert!(
        board["workUnits"]["AUTH-001"]
            .get("nextArchitectureNoteId")
            .is_none(),
        "{board}"
    );

    // @step And user adds new architecture note "Latest architectural decision"
    let result = dispatch(
        dir,
        "add-architecture-note",
        json!({"workUnitId": "AUTH-001", "note": "Latest architectural decision"}),
    );
    succeeded("add-architecture-note", &result)?;
    let board = read_board(dir)?;
    let notes = board["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // @step Then new note should get id = 3 (not id = 0)
    let added = notes
        .iter()
        .find(|n| n["text"] == "Latest architectural decision")
        .ok_or("the new note is missing")?;
    assert_eq!(added["id"], json!(3));

    // @step And no ID collision should occur with existing notes at indices 0, 1, 2
    assert_eq!(note_ids(&board), vec![0, 1, 2, 3]);
    Ok(())
}

#[test]
fn preserve_the_done_order_through_auto_compact() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given work units BOARD-003, BOARD-001 and BOARD-002 are in the done state in that order
    // @step And work unit BOARD-004 is validating with a soft-deleted rule
    let rules = json!([
        {"id": 0, "text": "Kept rule", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"},
        {"id": 1, "text": "Dropped rule", "deleted": true, "createdAt": "2026-11-01T00:00:00.000Z"}
    ]);
    write_board(
        dir,
        json!({
            "BOARD-001": unit("BOARD-001", "done", json!({})),
            "BOARD-002": unit("BOARD-002", "done", json!({})),
            "BOARD-003": unit("BOARD-003", "done", json!({})),
            "BOARD-004": unit("BOARD-004", "validating", json!({"rules": rules})),
        }),
        &[
            ("done", vec!["BOARD-003", "BOARD-001", "BOARD-002"]),
            ("validating", vec!["BOARD-004"]),
        ],
    )?;

    // @step When BOARD-004 is moved to done status with auto-compact
    let result = dispatch(
        dir,
        "update-work-unit-status",
        json!({"workUnitId": "BOARD-004", "status": "done"}),
    );
    succeeded("update-work-unit-status", &result)?;
    let board = read_board(dir)?;

    // @step Then the soft-deleted rule of BOARD-004 should be removed
    assert_eq!(
        board["workUnits"]["BOARD-004"]["rules"],
        json!([{"id": 0, "text": "Kept rule", "deleted": false, "createdAt": "2026-11-01T00:00:00.000Z"}])
    );

    // @step And the done state array in work-units.json should be BOARD-003, BOARD-001, BOARD-002, BOARD-004
    assert_eq!(
        board["states"]["done"],
        json!(["BOARD-003", "BOARD-001", "BOARD-002", "BOARD-004"])
    );
    assert_eq!(board["states"]["validating"], json!([]));
    Ok(())
}
