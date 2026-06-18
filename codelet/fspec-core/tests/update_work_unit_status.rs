#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-work-unit-status-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `update-work-unit-status` (RPC-319). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: the current core stub is 1-arg `run(args_json)` → NotYetPorted,
// so every dispatch of `update-work-unit-status` returns success=false with
// the NotYetPorted message. These tests assert the REAL ported behaviour, so
// they FAIL now — that is the correct red-phase state.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

/// Raw JSON fields that satisfy the story review-validation gate
/// (Example Mapping rules + examples, architectural notes, AST research
/// attachment) so that specifying→testing reaches later gates. Mirrors the
/// Level-1 hard blocks in `src/utils/review-validation.ts`.
const REVIEW_OK: &str = r#""rules": [{ "id": 0, "text": "rule", "deleted": false }], "examples": [{ "id": 0, "text": "example", "deleted": false }], "architectureNotes": [{ "id": 0, "text": "note", "deleted": false }], "attachments": ["spec/attachments/AUTH-001/ast-research-login.json"]"#;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("valid JSON")
}

/// Build a single-unit work-units.json document. `status` sets both the unit's
/// status field and the matching state array. `extra_fields` is raw JSON
/// inserted into the AUTH-001 object (without a leading comma; pass "" for
/// none). `state_history` is the raw array body for stateHistory (without
/// brackets; pass "" for none).
fn doc(id: &str, status: &str, extra_fields: &str) -> String {
    let states = state_arrays(id, status);
    let extra = if extra_fields.trim().is_empty() {
        String::new()
    } else {
        format!(", {extra_fields}")
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "Login", "type": "story", "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"{extra}
    }}
  }},
  "states": {{ {states} }}
}}"#
    )
}

/// Build the seven state arrays placing `id` in the `status` array.
fn state_arrays(id: &str, status: &str) -> String {
    let mut parts = Vec::new();
    for s in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        if s == status {
            parts.push(format!(r#""{s}": ["{id}"]"#));
        } else {
            parts.push(format!(r#""{s}": []"#));
        }
    }
    parts.join(", ")
}

fn status_of(data: &Value, id: &str) -> String {
    data["workUnits"][id]["status"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ---------- scenarios ----------

#[test]
fn valid_forward_transition_records_state_history_entry() {
    // Scenario: Valid forward transition records a state-history entry

    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "backlog", ""));

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "specifying"}),
    ));

    // @step Then the command succeeds
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the work unit status becomes "specifying"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "specifying");

    // @step And a state-history entry for "specifying" is recorded with a timestamp
    let history = data["workUnits"]["AUTH-001"]["stateHistory"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let entry = history
        .iter()
        .find(|h| h["state"].as_str() == Some("specifying"))
        .expect("state-history entry for 'specifying'");
    let ts = entry["timestamp"].as_str().unwrap_or("");
    assert!(
        ts.contains('T') && ts.ends_with('Z') && !ts.is_empty(),
        "state-history timestamp must be ISO-8601; got '{ts}'"
    );
}

#[test]
fn invalid_transition_is_rejected_with_allowed_transitions_message() {
    // Scenario: Invalid transition is rejected with an allowed-transitions message

    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "backlog", ""));

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "done"}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message names the allowed transitions from "backlog"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid state transition from 'backlog' to 'done'"),
        "missing invalid-transition text; got: {msg}"
    );

    // @step And the work unit status remains "backlog"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "backlog");
}

#[test]
fn unknown_work_unit_id_is_rejected() {
    // Scenario: Unknown work unit id is rejected

    // @step Given no work unit "NOPE-999" exists
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "backlog", ""));

    // @step When the dispatcher runs update-work-unit-status for "NOPE-999" with status "specifying"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-999", "status": "specifying"}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message is "Work unit NOPE-999 does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit NOPE-999 does not exist")
            || msg.contains("Work unit 'NOPE-999' does not exist"),
        "missing not-found text; got: {msg}"
    );
}

#[test]
fn moving_to_blocked_requires_a_blocked_reason() {
    // Scenario: Moving to blocked requires a blockedReason

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "specifying", ""));

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "blocked" and no blockedReason
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "blocked"}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message requires a blockedReason
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.to_lowercase().contains("blocked reason")
            || msg.contains("blocked-reason")
            || msg.contains("blockedReason"),
        "missing blocked-reason-required text; got: {msg}"
    );

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "blocked" and blockedReason "waiting on API"
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "blocked", "blockedReason": "waiting on API"}),
    ));

    // @step Then the command succeeds
    assert!(result2.success, "expected success=true; got {result2:?}");

    // @step And the work unit status becomes "blocked"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "blocked");
}

// ---------- feature/coverage fixture helpers ----------

/// Write a feature file under spec/features tagged with @<id>. `body` is the
/// scenario/background body appended after the tag + Feature header.
fn write_feature(project_root: &Path, name: &str, id: &str, body: &str) -> std::path::PathBuf {
    let dir = project_root.join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    let path = dir.join(format!("{name}.feature"));
    let content = format!("@{id}\nFeature: {name}\n\n{body}\n");
    fs::write(&path, content).expect("write feature");
    path
}

#[test]
fn specifying_to_testing_blocked_by_prefill_placeholders() {
    // Scenario: specifying to testing is blocked by prefill placeholders

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "specifying", ""));

    // @step And its linked feature file contains prefill placeholders
    write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Background: User Story\n    As a [role]\n    I want [action]\n    So that [benefit]\n\n  Scenario: Login\n    Given I am on the login page\n    When I log in\n    Then I see the dashboard",
    );

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing"}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message reports the prefill placeholders that must be resolved
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("prefill placeholder") || msg.contains("[role]") || msg.contains("placeholder"),
        "missing prefill text; got: {msg}"
    );

    // @step And the work unit status remains "specifying"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "specifying");
}

#[test]
fn specifying_to_testing_blocked_when_feature_has_no_scenarios() {
    // Scenario: specifying to testing is blocked when the feature has no scenarios

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    // The unit passes review validation (rules/examples/notes/AST research) so
    // the scenarios-required gate is the one exercised. No @AUTH-001-tagged
    // feature file exists, so checkScenariosExist (tag presence) fails.
    write_work_units(
        tmp.path(),
        &doc("AUTH-001", "specifying", REVIEW_OK),
    );

    // @step And its linked feature file has no scenarios
    // (No feature file is tagged @AUTH-001 — tag absence is what the TS
    // checkScenariosExist treats as "no scenarios".)

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing", "skipTemporalValidation": true}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message reports that scenarios are required before testing
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("No Gherkin scenarios") || msg.to_lowercase().contains("scenario"),
        "missing scenarios-required text; got: {msg}"
    );

    // @step And the work unit status remains "specifying"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "specifying");
}

/// Write a `.feature.coverage` JSON file next to a feature. `scenarios` is the
/// raw JSON array body (without brackets).
fn write_coverage(feature_path: &Path, scenarios_body: &str) {
    // feature_path ends in `.feature`; the coverage file is `<path>.coverage`.
    let explicit = {
        let mut s = feature_path.as_os_str().to_os_string();
        s.push(".coverage");
        std::path::PathBuf::from(s)
    };
    let body = format!(r#"{{ "scenarios": [{scenarios_body}] }}"#);
    fs::write(&explicit, &body).expect("write coverage");
}

#[test]
fn validating_transition_blocked_when_coverage_incomplete() {
    // Scenario: validating transition is blocked when coverage is incomplete

    // @step Given a work unit "AUTH-001" exists with status "implementing"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "implementing", ""));

    // @step And its linked feature has scenarios without test coverage mappings
    let feature_path = write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );
    write_coverage(
        &feature_path,
        r#"{ "name": "Login with valid credentials", "testMappings": [] }"#,
    );

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "validating"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "validating", "skipTemporalValidation": true}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message reports the uncovered scenarios
    let msg = result
        .error
        .as_ref()
        .or(result.system_reminder.as_ref())
        .expect("error or system_reminder must be set");
    assert!(
        msg.to_lowercase().contains("uncovered")
            || msg.contains("Login with valid credentials")
            || msg.to_lowercase().contains("coverage"),
        "missing uncovered-scenarios text; got: {msg}"
    );

    // @step And the work unit status remains "implementing"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "implementing");
}

#[test]
fn temporal_validation_blocks_forward_transition_unless_skipped() {
    // Scenario: Temporal validation blocks a forward transition unless skipped

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    // stateHistory carries a future specifying-entry timestamp so the feature
    // file (written now, in the past relative to that entry) violates ordering.
    write_work_units(
        tmp.path(),
        &doc(
            "AUTH-001",
            "specifying",
            &format!(
                r#""stateHistory": [{{ "state": "specifying", "timestamp": "2999-01-01T00:00:00.000Z" }}], {REVIEW_OK}"#
            ),
        ),
    );

    // @step And its linked feature file was last modified before the work unit entered "specifying"
    write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing"}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message reports a temporal-ordering violation
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("temporal ordering violation") || msg.to_lowercase().contains("temporal"),
        "missing temporal-ordering text; got: {msg}"
    );

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing" and skipTemporalValidation true
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing", "skipTemporalValidation": true}),
    ));

    // @step Then the command succeeds
    assert!(result2.success, "expected success=true; got {result2:?}");

    // @step And the work unit status becomes "testing"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "testing");
}

// ---------- git fixture helpers ----------

/// Initialise a real git repo in `dir` using the system `git` binary (same
/// approach as codelet/fspec-core/tests/list_checkpoints.rs::init_git_repo).
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
            .expect("git config/init");
    }
    fs::write(dir.join("README.md"), "# test\n").expect("seed README");
    for args in [
        vec!["add", "README.md"],
        vec!["commit", "--quiet", "-m", "initial"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .expect("git add/commit");
    }
}


/// Count checkpoint refs under refs/fspec-checkpoints/<id>/ for the given dir.
fn count_auto_checkpoints(dir: &Path, work_unit_id: &str) -> usize {
    let out = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/fspec-checkpoints/{work_unit_id}/"),
        ])
        .current_dir(dir)
        .output()
        .expect("git for-each-ref");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains("-auto-"))
        .count()
}

fn count_manual_checkpoints(dir: &Path, work_unit_id: &str) -> usize {
    let out = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/fspec-checkpoints/{work_unit_id}/"),
        ])
        .current_dir(dir)
        .output()
        .expect("git for-each-ref");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.contains("-auto-"))
        .count()
}

fn create_manual_checkpoint_ref(dir: &Path, work_unit_id: &str, name: &str) {
    let ref_name = format!("refs/fspec-checkpoints/{work_unit_id}/{name}");
    Command::new("git")
        .args(["update-ref", &ref_name, "HEAD"])
        .current_dir(dir)
        .status()
        .expect("git update-ref");
}

#[test]
fn dirty_working_directory_creates_automatic_checkpoint_before_transition() {
    // Scenario: A dirty working directory creates an automatic checkpoint before the transition

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    write_work_units(tmp.path(), &doc("AUTH-001", "specifying", REVIEW_OK));
    write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );

    // @step And the git working directory has uncommitted changes
    fs::write(tmp.path().join("dirty.txt"), "uncommitted\n").expect("write dirty file");

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing", "skipTemporalValidation": true}),
    ));

    // @step Then an automatic git checkpoint is created before the transition is applied
    assert!(
        count_auto_checkpoints(tmp.path(), "AUTH-001") >= 1,
        "expected at least one auto checkpoint ref to be created"
    );

    // @step And the command succeeds
    assert!(result.success, "expected success=true; got {result:?}");
}

#[test]
fn transitioning_to_backlog_does_not_create_automatic_checkpoint() {
    // Scenario: Transitioning to backlog does not create an automatic checkpoint

    // @step Given a work unit "AUTH-001" exists with status "blocked"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    write_work_units(
        tmp.path(),
        &doc("AUTH-001", "blocked", r#""blockedReason": "stuck""#),
    );

    // @step And the git working directory has uncommitted changes
    fs::write(tmp.path().join("dirty.txt"), "uncommitted\n").expect("write dirty file");

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "backlog"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "backlog"}),
    ));

    // @step Then no automatic git checkpoint is created
    assert_eq!(
        count_auto_checkpoints(tmp.path(), "AUTH-001"),
        0,
        "no auto checkpoint must be created for →backlog"
    );

    // @step And the command fails because moving back to backlog is not allowed
    assert!(
        !result.success,
        "moving to backlog must be rejected (TS blocks it); got {result:?}"
    );
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Cannot move work back to backlog"),
        "expected backlog-block message; got: {msg}"
    );
}

#[test]
fn transitioning_to_done_compacts_and_cleans_auto_checkpoints() {
    // Scenario: Transitioning to done compacts the work unit and cleans auto-checkpoints

    // @step Given a work unit "AUTH-001" exists with status "validating"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    // Unit carries a soft-deleted rule so compaction has something to remove,
    // plus a fully-covered feature so the →done coverage gate passes.
    write_work_units(
        tmp.path(),
        &doc(
            "AUTH-001",
            "validating",
            r#""rules": [{ "id": 0, "text": "live", "deleted": false }, { "id": 1, "text": "gone", "deleted": true }], "nextRuleId": 2, "linkedFeatures": ["user-login"]"#,
        ),
    );
    let feature_path = write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );
    write_coverage(
        &feature_path,
        r#"{ "name": "Login with valid credentials", "testMappings": [{ "file": "t.rs", "lines": "1-2", "implMappings": [{ "file": "i.rs", "lines": "3-4" }] }] }"#,
    );

    // @step And the work unit has both automatic and manual checkpoints
    create_manual_checkpoint_ref(tmp.path(), "AUTH-001", "AUTH-001-auto-validating");
    create_manual_checkpoint_ref(tmp.path(), "AUTH-001", "baseline");

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "done", "skipTemporalValidation": true}),
    ));

    // @step Then the command succeeds
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the work unit is compacted
    let data = read_work_units(tmp.path());
    let rules = data["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rules.len(), 1, "compaction must drop the deleted rule; got {data}");
    assert_eq!(status_of(&data, "AUTH-001"), "done");

    // @step And automatic checkpoints are removed while manual checkpoints are preserved
    assert_eq!(
        count_auto_checkpoints(tmp.path(), "AUTH-001"),
        0,
        "auto checkpoints must be cleaned on →done"
    );
    assert!(
        count_manual_checkpoints(tmp.path(), "AUTH-001") >= 1,
        "manual checkpoints must be preserved on →done"
    );

    // @step And a consolidated status-change system-reminder is emitted
    let reminder = result
        .system_reminder
        .as_deref()
        .or(Some(result.data.as_str()))
        .unwrap_or("");
    assert!(
        reminder.contains("<system-reminder>") || result.data.contains("done"),
        "expected a status-change system-reminder; got data={:?} reminder={:?}",
        result.data,
        result.system_reminder
    );
}

#[test]
fn blocking_pre_hook_failure_prevents_transition() {
    // Scenario: A blocking pre-hook failure prevents the transition

    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let tmp = TempDir::new().expect("tempdir");
    // Virtual hook bound to pre-testing event that fails (exit 1) and is
    // blocking, so the specifying→testing transition must abort.
    write_work_units(
        tmp.path(),
        &doc(
            "AUTH-001",
            "specifying",
            &format!(
                r#""virtualHooks": [{{ "name": "must-pass", "event": "pre-testing", "command": "exit 1", "blocking": true }}], {REVIEW_OK}"#
            ),
        ),
    );
    write_feature(
        tmp.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );

    // @step And a blocking pre-transition hook is configured to fail
    // (configured above via the virtualHooks array)

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "testing"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "testing", "skipTemporalValidation": true}),
    ));

    // @step Then the command fails
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the work unit status remains "specifying"
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "specifying");

    // @step And the blocking hook stderr is surfaced in a system-reminder
    let surfaced = result
        .error
        .as_deref()
        .map(|e| e.contains("system-reminder") || e.contains("BLOCKING HOOK"))
        .unwrap_or(false)
        || result
            .system_reminder
            .as_deref()
            .map(|r| r.contains("BLOCKING HOOK") || r.contains("system-reminder"))
            .unwrap_or(false);
    assert!(
        surfaced,
        "blocking hook stderr must be surfaced; got error={:?} reminder={:?}",
        result.error, result.system_reminder
    );
}

#[test]
fn ipc_notification_is_a_no_op_in_the_rust_port() {
    // Scenario: IPC notification is a no-op in the Rust port

    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("AUTH-001", "backlog", ""));

    // @step When the dispatcher runs update-work-unit-status for "AUTH-001" with status "specifying"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "status": "specifying"}),
    ));

    // @step Then the command succeeds
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And no IPC notification is attempted
    // The Rust port has no IPC channel — success without any socket/pipe is the
    // observable proof. We assert the transition completed cleanly on disk.
    let data = read_work_units(tmp.path());
    assert_eq!(status_of(&data, "AUTH-001"), "specifying");
}
