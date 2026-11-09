#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/global-status-hooks-run-for-every-work-unit.feature
//
// HOOK-020: update-work-unit-status runs the pre-<status> / post-<status>
// hooks of spec/fspec-hooks.json for every work unit, with the contract of
// virtual status hooks. Hook commands are inline shell commands (no script
// files are written and executed), so no test races a file still open for
// writing.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

/// A board holding task AUTH-001 in `implementing` with `virtual_hooks`.
fn board(root: &Path, virtual_hooks: Value) {
    fs::create_dir_all(root.join("spec")).unwrap();
    let board = json!({
        "meta": {"version": "1.0.0", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": {"AUTH-001": {
            "id": "AUTH-001", "title": "Login", "type": "task", "status": "implementing",
            "createdAt": "2026-11-07T00:00:00.000Z", "updatedAt": "2026-11-07T00:00:00.000Z",
            "stateHistory": [
                {"state": "backlog", "timestamp": "2026-11-07T00:00:00.000Z"},
                {"state": "specifying", "timestamp": "2026-11-07T00:00:01.000Z"},
                {"state": "implementing", "timestamp": "2026-11-07T00:00:02.000Z"}
            ],
            "virtualHooks": virtual_hooks
        }},
        "states": {"backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-001"],
                   "validating": [], "done": [], "blocked": []}
    });
    fs::write(root.join("spec/work-units.json"), board.to_string()).unwrap();
}

fn global_hooks(root: &Path, hooks: Value) {
    fs::write(
        root.join("spec/fspec-hooks.json"),
        json!({ "hooks": hooks }).to_string(),
    )
    .unwrap();
}

fn move_to(root: &Path, args: Value) -> DispatchResult {
    let mut args = args;
    args["workUnitId"] = json!("AUTH-001");
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn status(root: &Path) -> String {
    let raw = fs::read_to_string(root.join("spec/work-units.json")).unwrap();
    let board: Value = serde_json::from_str(&raw).unwrap();
    board["workUnits"]["AUTH-001"]["status"]
        .as_str()
        .unwrap()
        .to_string()
}

fn error(result: &DispatchResult) -> String {
    format!(
        "{}{}",
        result.error.clone().unwrap_or_default(),
        result.system_reminder.clone().unwrap_or_default()
    )
}

#[test]
fn a_failing_blocking_global_post_status_hook_blocks_the_move() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given spec/fspec-hooks.json has a blocking post-implementing hook "lint" that exits 1
    board(root, json!([]));
    global_hooks(
        root,
        json!({"post-implementing": [
            {"name": "lint", "command": "echo lint failed >&2; exit 1", "blocking": true}]}),
    );
    // @step And work unit "AUTH-001" is implementing
    assert_eq!(status(root), "implementing");

    // @step When I run "fspec update-work-unit-status AUTH-001 validating"
    let result = move_to(root, json!({"status": "validating"}));

    // @step Then the command fails with "BLOCKING HOOK FAILURE: global hook 'lint' for AUTH-001 failed at post-implementing."
    assert!(!result.success, "the move must be blocked: {result:?}");
    let error = error(&result);
    assert!(
        error.contains(
            "BLOCKING HOOK FAILURE: global hook 'lint' for AUTH-001 failed at post-implementing."
        ),
        "{error}"
    );
    assert!(error.contains("Exit code: 1"), "{error}");
    assert!(error.contains("lint failed"), "{error}");
    // @step And work unit "AUTH-001" is still implementing
    assert_eq!(status(root), "implementing");
}

#[test]
fn global_and_virtual_status_hooks_run_in_event_order() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given a global post-implementing hook "p" and a global pre-validating hook "g" that append their name to a file
    global_hooks_after_board(
        root,
        json!([{"name": "v", "event": "post-implementing", "command": "cat > virtual.json; echo v >> order.txt"}]),
        json!({
            "pre-validating": [{"name": "g", "command": "echo g >> order.txt"}],
            "post-implementing": [{"name": "p", "command": "echo p >> order.txt"}]
        }),
    );
    // @step And work unit "AUTH-001" has a virtual post-implementing hook "v" that appends its name to the file
    assert!(fs::read_to_string(root.join("spec/work-units.json"))
        .unwrap()
        .contains("echo v >> order.txt"));

    // @step When work unit "AUTH-001" moves from implementing to validating
    let result = move_to(root, json!({"status": "validating"}));

    // @step Then the file lists "p", "v" and "g" in that order
    assert!(result.success, "{result:?}");
    let virtual_context: Value =
        serde_json::from_str(&fs::read_to_string(root.join("virtual.json")).unwrap()).unwrap();
    assert_eq!(virtual_context["fromStatus"], "implementing");
    assert_eq!(virtual_context["toStatus"], "validating");
    assert_eq!(
        fs::read_to_string(root.join("order.txt")).unwrap(),
        "p\nv\ng\n"
    );
    // @step And work unit "AUTH-001" is validating
    assert_eq!(status(root), "validating");
}

fn global_hooks_after_board(root: &Path, virtual_hooks: Value, hooks: Value) {
    board(root, virtual_hooks);
    global_hooks(root, hooks);
}

#[test]
fn a_global_status_hook_receives_the_work_unit_context_and_honours_its_condition() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    board(root, json!([]));
    // @step Given a global pre-validating hook that saves its stdin
    // @step And a global pre-validating hook with condition prefix "BUG" that writes a marker file
    global_hooks(
        root,
        json!({"pre-validating": [
            {"name": "context", "command": "cat > stdin.json"},
            {"name": "bugs-only", "command": "touch marker", "condition": {"prefix": ["BUG"]}}
        ]}),
    );

    // @step When work unit "AUTH-001" moves from implementing to validating
    let result = move_to(root, json!({"status": "validating"}));

    // @step Then the saved stdin has workUnitId "AUTH-001", event "pre-validating", fromStatus "implementing" and toStatus "validating"
    assert!(result.success, "{result:?}");
    let context: Value =
        serde_json::from_str(&fs::read_to_string(root.join("stdin.json")).unwrap()).unwrap();
    assert_eq!(context["workUnitId"], "AUTH-001");
    assert_eq!(context["event"], "pre-validating");
    assert_eq!(context["fromStatus"], "implementing");
    assert_eq!(context["toStatus"], "validating");
    assert!(context["timestamp"].is_string(), "{context}");
    // @step And the marker file does not exist
    assert!(!root.join("marker").exists());
}

#[test]
fn a_move_to_blocked_runs_no_global_status_hooks() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    // @step Given a global post-implementing hook and a global pre-blocked hook that write marker files
    board(root, json!([]));
    global_hooks(
        root,
        json!({
            "post-implementing": [{"name": "leaving", "command": "touch left"}],
            "pre-blocked": [{"name": "entering", "command": "touch entered"}]
        }),
    );

    // @step When work unit "AUTH-001" moves from implementing to blocked
    let result = move_to(
        root,
        json!({"status": "blocked", "blockedReason": "waiting on the API"}),
    );

    // @step Then neither marker file exists
    assert!(result.success, "{result:?}");
    assert_eq!(status(root), "blocked");
    assert!(!root.join("left").exists());
    assert!(!root.join("entered").exists());
}

#[test]
fn an_unparseable_hooks_file_blocks_a_status_move() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    board(root, json!([]));
    // @step Given spec/fspec-hooks.json is not valid JSON
    fs::write(root.join("spec/fspec-hooks.json"), "{ \"hooks\": ").unwrap();

    // @step When work unit "AUTH-001" moves from implementing to validating
    let result = move_to(root, json!({"status": "validating"}));

    // @step Then the command fails naming "Invalid JSON in spec/fspec-hooks.json"
    assert!(!result.success, "{result:?}");
    assert!(
        error(&result).contains("Invalid JSON in spec/fspec-hooks.json"),
        "{}",
        error(&result)
    );
    // @step And work unit "AUTH-001" is still implementing
    assert_eq!(status(root), "implementing");
}
