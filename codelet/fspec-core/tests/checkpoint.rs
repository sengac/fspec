#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/checkpoint-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `checkpoint`
// (RPC-202). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim. Tests drive the command
// through the shared dispatcher (codelet_fspec_core::dispatch_command),
// which is the same front door the agent loop uses.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "checkpoint".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn req_raw(project_root: &Path, args_json: &str) -> DispatchRequest {
    DispatchRequest {
        command: "checkpoint".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Initialise a real git repo with a single committed README (clean tree).
fn init_git_repo(dir: &Path) {
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(dir)
        .status()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .status()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .status()
        .expect("git config name");
    fs::write(dir.join("README.md"), "# test\n").expect("seed README");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .status()
        .expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", "initial"])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

/// Add `n` brand-new untracked files so the working tree differs from HEAD.
fn dirty_with_n_files(dir: &Path, n: usize) {
    for i in 0..n {
        fs::write(dir.join(format!("change-{i}.txt")), format!("change {i}\n"))
            .expect("write dirty file");
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn capture_a_dirty_working_tree_and_render_the_success_banner() {
    // Scenario: Capture a dirty working tree and render the success banner

    // @step Given a git repository with 3 uncommitted file changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    dirty_with_n_files(tmp.path(), 3);

    // @step And the dispatcher receives command "checkpoint" with workUnitId "AUTH-001" and checkpointName "baseline"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline" }),
    ));

    // @step When fspec_core::commands::checkpoint::run executes with project_root set to that repository
    // (the dispatcher routes through commands::checkpoint::run)

    // @step Then the result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the rendered text contains "✓ Created checkpoint \"baseline\" for AUTH-001"
    assert!(
        result
            .data
            .contains("\u{2713} Created checkpoint \"baseline\" for AUTH-001"),
        "missing success banner; got:\n{}",
        result.data
    );

    // @step And the rendered text contains "Captured 3 file(s)"
    assert!(
        result.data.contains("Captured 3 file(s)"),
        "missing captured-count line; got:\n{}",
        result.data
    );
}

#[test]
fn persist_the_metadata_index_on_successful_capture() {
    // Scenario: Persist the metadata index on successful capture

    // @step Given a git repository with uncommitted changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    dirty_with_n_files(tmp.path(), 1);

    // @step When fspec_core::commands::checkpoint::run captures a checkpoint named "baseline" for "AUTH-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the file ".git/fspec-checkpoints-index/AUTH-001.json" exists
    let index_path = tmp
        .path()
        .join(".git/fspec-checkpoints-index/AUTH-001.json");
    assert!(
        index_path.exists(),
        "expected index file at {}",
        index_path.display()
    );

    // @step And it contains a checkpoints entry whose name is "baseline" with a sha and an ISO-8601 timestamp
    let raw = fs::read_to_string(&index_path).expect("read index file");
    let parsed = parse_data(&raw);
    let entry = parsed["checkpoints"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|cp| cp.get("name").and_then(|n| n.as_str()) == Some("baseline"))
        })
        .expect("baseline checkpoints entry");
    assert!(
        entry["sha"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "baseline entry must carry a non-empty sha; got {entry:?}"
    );
    let ts = entry["timestamp"].as_str().expect("timestamp string");
    assert!(
        ts.len() >= 20,
        "timestamp must be ISO-8601 (length >= 20); got {ts:?}"
    );

    // @step And the JSON is pretty-printed with 2-space indentation
    assert!(
        raw.lines()
            .any(|l| l.starts_with("  \"checkpoints\"") || l.starts_with("  \"checkpoints\":")),
        "expected a 2-space-indented `checkpoints` line; got:\n{raw}"
    );
}

#[test]
fn clean_working_tree_captures_nothing_and_reports_failure() {
    // Scenario: Clean working tree captures nothing and reports failure

    // @step Given a git repository with no uncommitted changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());

    // @step When fspec_core::commands::checkpoint::run attempts to capture "baseline" for "AUTH-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "format": "json" }),
    ));

    // @step Then the result reports success false with an empty capturedFiles list
    assert!(result.success, "dispatcher should not error: {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "expected success=false in payload; got {}",
        result.data
    );
    assert_eq!(
        data["capturedFiles"].as_array().map(Vec::len),
        Some(0),
        "expected empty capturedFiles; got {}",
        result.data
    );

    // @step And no ".git/fspec-checkpoints-index/AUTH-001.json" file is written
    assert!(
        !tmp.path()
            .join(".git/fspec-checkpoints-index/AUTH-001.json")
            .exists(),
        "no index file must be written on a clean working tree"
    );
}

#[test]
fn reject_an_empty_checkpoint_name() {
    // Scenario: Reject an empty checkpoint name

    // @step Given the dispatcher receives command "checkpoint" with workUnitId "AUTH-001" and an empty checkpointName
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());

    // @step When fspec_core::commands::checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "" }),
    ));

    // @step Then it returns an InvalidArgs error naming the empty checkpointName field
    assert!(!result.success, "expected failure, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("checkpointName"),
        "error message missing 'checkpointName' substring: {msg}"
    );
}

#[test]
fn reject_a_missing_work_unit_id() {
    // Scenario: Reject a missing work unit id

    // @step Given the dispatcher receives command "checkpoint" with no workUnitId field
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());

    // @step When fspec_core::commands::checkpoint::run executes
    let result = dispatch_command(req_raw(tmp.path(), r#"{"checkpointName":"baseline"}"#));

    // @step Then it returns an InvalidArgs error naming the missing workUnitId field
    assert!(!result.success, "expected failure, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("workUnitId"),
        "error message missing 'workUnitId' substring: {msg}"
    );
}

#[test]
fn format_json_emits_structured_payload_preserving_key_order() {
    // Scenario: format json emits the structured payload preserving key order

    // @step Given a git repository with uncommitted changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    dirty_with_n_files(tmp.path(), 2);

    // @step And the dispatcher receives command "checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and format "json"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "format": "json" }),
    ));

    // @step When fspec_core::commands::checkpoint::run executes
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output is pretty-printed JSON
    let data = parse_data(&result.data);
    assert!(
        result.data.contains('\n') && result.data.contains("  "),
        "expected pretty-printed (multi-line, indented) JSON; got:\n{}",
        result.data
    );

    // @step And the object keys are "success", "checkpointName", "capturedFiles", "includedUntracked" in that order
    let pos = |needle: &str| {
        result
            .data
            .find(needle)
            .unwrap_or_else(|| panic!("missing key {needle} in:\n{}", result.data))
    };
    let p_success = pos("\"success\"");
    let p_name = pos("\"checkpointName\"");
    let p_files = pos("\"capturedFiles\"");
    let p_untracked = pos("\"includedUntracked\"");
    assert!(
        p_success < p_name && p_name < p_files && p_files < p_untracked,
        "keys out of order; got:\n{}",
        result.data
    );

    // @step And "includedUntracked" is true
    assert_eq!(
        data["includedUntracked"].as_bool(),
        Some(true),
        "includedUntracked must be true; got {}",
        result.data
    );
}
