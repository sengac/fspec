#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/cleanup-checkpoints-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `cleanup-checkpoints`
// (RPC-203). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// NOTE (Phase B): the core command is still a NotYetPorted stub, so these
// green-phase assertions are EXPECTED to fail until Phase C wires the real
// implementation.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "cleanup-checkpoints".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn req_raw(project_root: &Path, args_json: &str) -> DispatchRequest {
    DispatchRequest {
        command: "cleanup-checkpoints".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn init_git_repo(dir: &Path) {
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .expect("git setup");
    }
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

fn create_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str) {
    let ref_name = format!("refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}");
    let status = Command::new("git")
        .args(["update-ref", &ref_name, "HEAD"])
        .current_dir(dir)
        .status()
        .expect("git update-ref");
    assert!(status.success(), "git update-ref {ref_name} HEAD failed");
}

fn write_index_file(dir: &Path, work_unit_id: &str, entries: &[(String, String)]) {
    let index_dir = dir.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&index_dir).expect("mkdir fspec-checkpoints-index");
    let arr: Vec<Value> = entries
        .iter()
        .map(|(name, ts)| json!({ "name": name, "sha": "deadbeef", "timestamp": ts }))
        .collect();
    let payload = json!({ "checkpoints": arr });
    fs::write(
        index_dir.join(format!("{work_unit_id}.json")),
        serde_json::to_string_pretty(&payload).unwrap(),
    )
    .expect("write index file");
}

/// Create `n` checkpoints named cp-00..cp-(n-1) with ascending timestamps
/// (cp-00 oldest, cp-(n-1) newest). Returns the (name,timestamp) list.
fn seed_checkpoints(dir: &Path, work_unit_id: &str, n: usize) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for i in 0..n {
        let name = format!("cp-{i:02}");
        // Minute-granular ascending timestamps within 2026-06-01.
        let ts = format!("2026-06-01T{:02}:{:02}:00.000Z", i / 60, i % 60);
        create_checkpoint(dir, work_unit_id, &name);
        entries.push((name, ts));
    }
    write_index_file(dir, work_unit_id, &entries);
    entries
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn delete_the_oldest_checkpoints_beyond_the_keep_last_window() {
    // Scenario: Delete the oldest checkpoints beyond the keepLast window

    // @step Given a git repository with 12 checkpoints for work unit "AUTH-001"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    let entries = seed_checkpoints(tmp.path(), "AUTH-001", 12);

    // @step And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 5
    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "keepLast": 5, "format": "json" }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result reports deletedCount 7 and preservedCount 5
    assert_eq!(data["deletedCount"].as_u64(), Some(7));
    assert_eq!(data["preservedCount"].as_u64(), Some(5));

    // @step And the 5 preserved checkpoints are the 5 newest by timestamp
    let newest_5: Vec<&str> = entries
        .iter()
        .rev()
        .take(5)
        .map(|(n, _)| n.as_str())
        .collect();
    let preserved: Vec<&str> = data["preserved"]
        .as_array()
        .expect("preserved array")
        .iter()
        .map(|e| e["name"].as_str().expect("name"))
        .collect();
    for name in &newest_5 {
        assert!(
            preserved.contains(name),
            "expected {name} preserved; got {preserved:?}"
        );
    }
}

#[test]
fn render_the_cleanup_summary_text() {
    // Scenario: Render the cleanup summary text

    // @step Given a git repository with 3 checkpoints for work unit "AUTH-001"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    seed_checkpoints(tmp.path(), "AUTH-001", 3);

    // @step And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 1
    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "keepLast": 1 }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered text contains "Cleaning up checkpoints for AUTH-001 (keeping last 1)"
    assert!(
        result
            .data
            .contains("Cleaning up checkpoints for AUTH-001 (keeping last 1)"),
        "missing header; got:\n{}",
        result.data
    );

    // @step And the rendered text contains "Deleted 2 checkpoint(s):"
    assert!(
        result.data.contains("Deleted 2 checkpoint(s):"),
        "missing deleted summary; got:\n{}",
        result.data
    );

    // @step And the rendered text contains "Preserved 1 checkpoint(s):"
    assert!(
        result.data.contains("Preserved 1 checkpoint(s):"),
        "missing preserved summary; got:\n{}",
        result.data
    );

    // @step And the rendered text contains "✓ Cleanup complete: 2 deleted, 1 preserved"
    assert!(
        result
            .data
            .contains("\u{2713} Cleanup complete: 2 deleted, 1 preserved"),
        "missing completion banner; got:\n{}",
        result.data
    );
}

#[test]
fn no_deletion_when_count_is_within_the_keep_last_window() {
    // Scenario: No deletion when count is within the keepLast window

    // @step Given a git repository with 3 checkpoints for work unit "BUG-003"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    seed_checkpoints(tmp.path(), "BUG-003", 3);

    // @step And the dispatcher receives command "cleanup-checkpoints" with workUnitId "BUG-003" and keepLast 10
    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "BUG-003", "keepLast": 10, "format": "json" }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result reports deletedCount 0 and preservedCount 3
    assert_eq!(data["deletedCount"].as_u64(), Some(0));
    assert_eq!(data["preservedCount"].as_u64(), Some(3));

    // @step And the rendered text contains "✓ Cleanup complete: 0 deleted, 3 preserved"
    // (json format omits banner; re-run in text to assert the rendered line)
    let text = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "BUG-003", "keepLast": 10 }),
    ));
    assert!(
        text.data
            .contains("\u{2713} Cleanup complete: 0 deleted, 3 preserved"),
        "missing completion banner; got:\n{}",
        text.data
    );
}

#[test]
fn reject_a_keep_last_of_zero() {
    // Scenario: Reject a keepLast of zero

    // @step Given the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 0
    let tmp = TempDir::new().expect("tempdir");

    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "keepLast": 0 }),
    ));

    // @step Then it returns an InvalidArgs error containing "--keep-last must be a positive number"
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("--keep-last must be a positive number"),
        "error must mention the positive-number constraint; got: {msg}"
    );
}

#[test]
fn reject_a_missing_work_unit_id() {
    // Scenario: Reject a missing work unit id

    // @step Given the dispatcher receives command "cleanup-checkpoints" with no workUnitId field and keepLast 5
    let tmp = TempDir::new().expect("tempdir");

    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req_raw(tmp.path(), r#"{"keepLast":5}"#));

    // @step Then it returns an InvalidArgs error naming the missing workUnitId field
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("workUnitId"),
        "error must name workUnitId; got: {msg}"
    );
}

#[test]
fn format_json_emits_the_structured_cleanup_payload() {
    // Scenario: format json emits the structured cleanup payload

    // @step Given a git repository with 4 checkpoints for work unit "AUTH-001"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    seed_checkpoints(tmp.path(), "AUTH-001", 4);

    // @step And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001", keepLast 2 and format "json"
    // @step When fspec_core::commands::cleanup_checkpoints::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "keepLast": 2, "format": "json" }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output is pretty-printed JSON
    let data = parse_data(&result.data);
    assert!(
        result.data.contains('\n') && result.data.contains("  "),
        "expected pretty-printed JSON; got:\n{}",
        result.data
    );

    // @step And it has the keys "workUnitId", "deletedCount", "preservedCount", "deleted", "preserved" in that order
    let keys: Vec<&str> = data
        .as_object()
        .expect("root object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "workUnitId",
            "deletedCount",
            "preservedCount",
            "deleted",
            "preserved"
        ],
        "key order mismatch; got {keys:?}"
    );

    // @step And "deleted" and "preserved" are arrays of objects with "name" and "timestamp" fields
    for key in ["deleted", "preserved"] {
        let arr = data[key]
            .as_array()
            .unwrap_or_else(|| panic!("{key} array"));
        for entry in arr {
            assert!(
                entry["name"].as_str().is_some(),
                "{key} entry missing name; got {entry:?}"
            );
            assert!(
                entry["timestamp"].as_str().is_some(),
                "{key} entry missing timestamp; got {entry:?}"
            );
        }
    }
}
