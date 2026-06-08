#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/list-checkpoints-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-checkpoints`
// (RPC-242). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-checkpoints".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn req_raw(project_root: &Path, args_json: &str) -> DispatchRequest {
    DispatchRequest {
        command: "list-checkpoints".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Initialise a real git repo in `dir` (uses the system `git` binary, same
/// approach as codelet/git/tests/common/mod.rs::setup_test_repo). gitoxide
/// can READ refs but writing checkpoint refs requires the gix object store
/// and a tracked file; the shell `git` binary is the path of least
/// resistance for fixture setup.
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

/// Create a checkpoint by writing the ref directly via shell `git
/// update-ref`. Equivalent to what `codelet_git::ghost_commit::
/// create_ghost_commit` would produce — the dispatcher only iterates
/// refs under `refs/fspec-checkpoints/{work_unit_id}/`, so any valid
/// commit OID will do (we point at HEAD). Avoids forcing
/// `codelet-fspec-core` to take a dev-dependency on `codelet-git`
/// just for fixture setup.
fn create_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str) {
    let ref_name = format!("refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}");
    let status = Command::new("git")
        .args(["update-ref", &ref_name, "HEAD"])
        .current_dir(dir)
        .status()
        .expect("git update-ref");
    assert!(status.success(), "git update-ref {ref_name} HEAD failed");
}

fn write_index_file(dir: &Path, work_unit_id: &str, entries: &[(&str, &str)]) {
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

fn write_raw_index_file(dir: &Path, work_unit_id: &str, raw: &str) {
    let index_dir = dir.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&index_dir).expect("mkdir fspec-checkpoints-index");
    fs::write(index_dir.join(format!("{work_unit_id}.json")), raw).expect("write raw index file");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn returns_empty_checkpoints_list_against_empty_tempdir_with_no_git_repo() {
    // Scenario: Returns empty checkpoints list against an empty tempdir with no git repository

    // @step Given an empty project root directory with no .git subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join(".git").exists());

    // @step When I dispatch the list-checkpoints command against that project root with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true with an empty checkpoints array
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["checkpoints"].as_array().map(Vec::len),
        Some(0),
        "expected empty checkpoints array, got {}",
        result.data
    );

    // @step Then the JSON data has a workUnitId field equal to 'AUTH-001'
    assert_eq!(data["workUnitId"].as_str(), Some("AUTH-001"));
}

#[test]
fn returns_text_sentinel_no_checkpoints_found_for_empty_results() {
    // Scenario: Returns text sentinel 'No checkpoints found' for empty results

    // @step Given an empty project root directory with no .git subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No checkpoints found for AUTH-001'
    assert_eq!(
        result.data, "No checkpoints found for AUTH-001",
        "expected exact text sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn missing_work_unit_id_field_fails_with_invalid_args() {
    // Scenario: Missing workUnitId field in args JSON fails with InvalidArgs

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the list-checkpoints command with the args JSON '{}'
    let result = dispatch_command(req_raw(tmp.path(), "{}"));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring 'workUnitId'
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("workUnitId"),
        "error message missing 'workUnitId' substring: {msg}"
    );
}

#[test]
fn empty_work_unit_id_string_fails_with_invalid_args() {
    // Scenario: Empty workUnitId string fails with InvalidArgs

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the list-checkpoints command with workUnitId=''
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring 'workUnitId'
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("workUnitId"),
        "error message missing 'workUnitId' substring: {msg}"
    );
}

#[test]
fn renders_single_manual_checkpoint_with_manual_icon_and_label() {
    // Scenario: Renders a single manual checkpoint with the manual icon and label

    // @step Given a git repository at the project root with a manual checkpoint named 'baseline' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "baseline");

    // @step Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    write_index_file(
        tmp.path(),
        "AUTH-001",
        &[("baseline", "2026-06-01T10:00:00.000Z")],
    );

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the substring 'Checkpoints for AUTH-001:'
    assert!(
        result.data.contains("Checkpoints for AUTH-001:"),
        "missing header; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring '📌  baseline (manual)'
    assert!(
        result.data.contains("\u{1F4CC}  baseline (manual)"),
        "missing manual icon+label line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring 'Created: 2026-06-01T10:00:00.000Z'
    assert!(
        result.data.contains("Created: 2026-06-01T10:00:00.000Z"),
        "missing Created line; got:\n{}",
        result.data
    );
}

#[test]
fn renders_single_automatic_checkpoint_with_automatic_icon_and_label() {
    // Scenario: Renders a single automatic checkpoint with the automatic icon and label

    // @step Given a git repository at the project root with an automatic checkpoint named 'AUTH-001-auto-testing' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "AUTH-001-auto-testing");

    // @step Given the checkpoint index file records timestamp '2026-06-02T12:00:00.000Z' for 'AUTH-001-auto-testing'
    write_index_file(
        tmp.path(),
        "AUTH-001",
        &[("AUTH-001-auto-testing", "2026-06-02T12:00:00.000Z")],
    );

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the substring '🤖  AUTH-001-auto-testing (automatic)'
    assert!(
        result
            .data
            .contains("\u{1F916}  AUTH-001-auto-testing (automatic)"),
        "missing automatic icon+label line; got:\n{}",
        result.data
    );
}

#[test]
fn json_format_sorts_checkpoints_by_timestamp_descending() {
    // Scenario: JSON format sorts checkpoints by timestamp descending

    // @step Given a git repository at the project root with checkpoints 'baseline' and 'AUTH-001-auto-testing' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "baseline");
    create_checkpoint(tmp.path(), "AUTH-001", "AUTH-001-auto-testing");

    // @step Given the checkpoint index file records 'baseline' at '2026-06-01T10:00:00.000Z' and 'AUTH-001-auto-testing' at '2026-06-02T12:00:00.000Z'
    write_index_file(
        tmp.path(),
        "AUTH-001",
        &[
            ("baseline", "2026-06-01T10:00:00.000Z"),
            ("AUTH-001-auto-testing", "2026-06-02T12:00:00.000Z"),
        ],
    );

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["checkpoints"].as_array().expect("checkpoints array");

    // @step Then the JSON checkpoints array has length 2
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");

    // @step Then the first entry has name='AUTH-001-auto-testing', isAutomatic=true, displayIcon='🤖'
    assert_eq!(arr[0]["name"].as_str(), Some("AUTH-001-auto-testing"));
    assert_eq!(arr[0]["isAutomatic"].as_bool(), Some(true));
    assert_eq!(arr[0]["displayIcon"].as_str(), Some("\u{1F916}"));

    // @step Then the second entry has name='baseline', isAutomatic=false, displayIcon='📌'
    assert_eq!(arr[1]["name"].as_str(), Some("baseline"));
    assert_eq!(arr[1]["isAutomatic"].as_bool(), Some(false));
    assert_eq!(arr[1]["displayIcon"].as_str(), Some("\u{1F4CC}"));
}

#[test]
fn json_format_emits_two_space_indent_with_canonical_field_set() {
    // Scenario: JSON format emits two-space indented payload with the canonical field set

    // @step Given a git repository at the project root with one manual checkpoint 'baseline' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "baseline");

    // @step Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    write_index_file(
        tmp.path(),
        "AUTH-001",
        &[("baseline", "2026-06-01T10:00:00.000Z")],
    );

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has workUnitId='AUTH-001' and a 'checkpoints' array of length 1
    let data = parse_data(&result.data);
    assert_eq!(data["workUnitId"].as_str(), Some("AUTH-001"));
    let arr = data["checkpoints"].as_array().expect("checkpoints array");
    assert_eq!(arr.len(), 1);

    // @step Then the first checkpoints entry contains fields name='baseline', timestamp='2026-06-01T10:00:00.000Z', displayIcon='📌', isAutomatic=false
    let entry = &arr[0];
    assert_eq!(entry["name"].as_str(), Some("baseline"));
    assert_eq!(
        entry["timestamp"].as_str(),
        Some("2026-06-01T10:00:00.000Z")
    );
    assert_eq!(entry["displayIcon"].as_str(), Some("\u{1F4CC}"));
    assert_eq!(entry["isAutomatic"].as_bool(), Some(false));

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"checkpoints\"")
            || l.starts_with("  \"workUnitId\"")),
        "expected a line starting with two-space indent + a root field; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected a four-space-indented `{{` line opening the checkpoints entry; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l.starts_with("      \"name\"")),
        "expected a line starting with six-space indent + \"name\"; got:\n{}",
        result.data
    );
}

#[test]
fn missing_index_file_falls_back_to_non_empty_iso8601_timestamp() {
    // Scenario: Missing index file falls back to a non-empty ISO-8601 timestamp

    // @step Given a git repository at the project root with a manual checkpoint 'baseline' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "baseline");

    // @step Given the file .git/fspec-checkpoints-index/AUTH-001.json does NOT exist
    assert!(!tmp
        .path()
        .join(".git/fspec-checkpoints-index/AUTH-001.json")
        .exists());

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the JSON checkpoints array has length 1
    let data = parse_data(&result.data);
    let arr = data["checkpoints"].as_array().expect("checkpoints array");
    assert_eq!(arr.len(), 1);

    // @step Then the baseline entry's timestamp is a non-empty string of length >= 20
    let ts = arr[0]["timestamp"].as_str().expect("timestamp string");
    assert!(
        ts.len() >= 20,
        "expected fallback timestamp of length >= 20 (ISO-8601 with millis); got: {ts:?}"
    );
}

#[test]
fn malformed_index_file_is_silently_swallowed() {
    // Scenario: Malformed index file is silently swallowed and falls back to a non-empty timestamp

    // @step Given a git repository at the project root with a manual checkpoint 'baseline' for AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_checkpoint(tmp.path(), "AUTH-001", "baseline");

    // @step Given the file .git/fspec-checkpoints-index/AUTH-001.json contains the malformed bytes '{ not json'
    write_raw_index_file(tmp.path(), "AUTH-001", "{ not json");

    // @step When I dispatch the list-checkpoints command with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed index file must be silently swallowed: {result:?}"
    );

    // @step Then the JSON checkpoints array has length 1
    let data = parse_data(&result.data);
    let arr = data["checkpoints"].as_array().expect("checkpoints array");
    assert_eq!(arr.len(), 1);

    // @step Then the baseline entry's timestamp is a non-empty string of length >= 20
    let ts = arr[0]["timestamp"].as_str().expect("timestamp string");
    assert!(
        ts.len() >= 20,
        "expected fallback timestamp of length >= 20; got: {ts:?}"
    );
}

#[test]
fn shared_infrastructure_wiring_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under codelet/fspec-core and codelet-git is wired as a dependency

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/Cargo.toml
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml =
        fs::read_to_string(crate_dir.join("Cargo.toml")).expect("Cargo.toml readable");

    // @step Then the dependencies section declares codelet-git via the workspace
    assert!(
        cargo_toml.contains("codelet-git"),
        "fspec-core/Cargo.toml must declare codelet-git; got:\n{cargo_toml}"
    );

    // @step When I inspect codelet/fspec-core/src/commands/list_checkpoints.rs
    let list_src = fs::read_to_string(crate_dir.join("src/commands/list_checkpoints.rs"))
        .expect("commands/list_checkpoints.rs readable");

    // @step Then it references codelet_git::ghost_commit::list_ghost_checkpoints
    assert!(
        list_src.contains("list_ghost_checkpoints"),
        "list_checkpoints.rs must reference codelet_git::ghost_commit::list_ghost_checkpoints; got:\n{list_src}"
    );

    // @step Then it references codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN
    assert!(
        list_src.contains("AUTO_CHECKPOINT_PATTERN"),
        "list_checkpoints.rs must reference codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN; got:\n{list_src}"
    );

    // @step Then it does NOT contain the substring 'FspecCoreError::NotYetPorted'
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "list_checkpoints.rs must no longer be a NotYetPorted stub; got:\n{list_src}"
    );
}
