#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/discover-foundation-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `discover-foundation`
// (RPC-226). Each scenario maps to exactly one #[test] with @step comments
// mirroring the Gherkin steps verbatim.
//
// RED PHASE: the command is still a stub returning NotYetPorted, so these
// tests FAIL now. They assert the real expected behaviour the Phase C
// implementation must satisfy.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "discover-foundation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_draft_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), body).expect("write draft");
}

fn write_foundation_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), body).expect("write foundation.json");
}

/// A fully-filled, schema-valid draft (no [QUESTION:]/[DETECTED:] markers).
fn valid_filled_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "Acme", "vision": "Ship faster", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": { "title": "Pain", "description": "Real pain", "impact": "high" }
        },
        "solutionSpace": {
            "overview": "A CLI",
            "capabilities": [ { "name": "Cap", "description": "Does things" } ]
        },
        "personas": [ { "name": "Dev", "description": "Builds", "goals": ["Ship"] } ]
    })
}

/// The canonical draft template `discover-foundation` writes on creation.
fn placeholder_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "[QUESTION: What problem does this solve?]",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).expect("read file")
}

// ---------- scenarios ----------

#[test]
fn fresh_discovery_creates_draft_with_placeholders_and_first_field_reminder() {
    // @step Given an empty project root tempdir with no spec/foundation.json.draft and no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And spec/foundation.json.draft exists on disk
    let draft_path = tmp.path().join("spec/foundation.json.draft");
    assert!(draft_path.exists(), "draft must be created");

    // @step And the draft on disk contains the version "2.0.0"
    let draft = read_to_string(&draft_path);
    assert!(
        draft.contains("\"2.0.0\""),
        "draft must contain version: {draft}"
    );

    // @step And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    assert!(
        draft.contains("[QUESTION: What is the project name?]"),
        "missing name placeholder"
    );

    // @step And the draft on disk contains the placeholder "[DETECTED: cli-tool]"
    assert!(
        draft.contains("[DETECTED: cli-tool]"),
        "missing detected placeholder"
    );

    // @step And the returned systemReminder contains "Field 1/8: project.name"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("Field 1/8: project.name"),
        "reminder: {reminder}"
    );
}

#[test]
fn rerun_without_force_when_draft_exists_is_blocked() {
    // @step Given a project root tempdir that already has a spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&placeholder_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let pre = read_to_string(&tmp.path().join("spec/foundation.json.draft"));

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned systemReminder contains "ERROR: foundation.json.draft already exists!"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("ERROR: foundation.json.draft already exists!"),
        "reminder: {reminder}"
    );

    // @step And spec/foundation.json.draft on disk is byte-equal to its pre-call contents
    let post = read_to_string(&tmp.path().join("spec/foundation.json.draft"));
    assert_eq!(pre, post, "draft must be unchanged");
}

#[test]
fn running_without_force_when_foundation_exists_is_blocked() {
    // @step Given a project root tempdir that has a spec/foundation.json but no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_foundation_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with no flags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned systemReminder contains "ERROR: foundation.json already exists!"
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("ERROR: foundation.json already exists!"),
        "reminder: {reminder}"
    );

    // @step And no spec/foundation.json.draft is created
    assert!(
        !tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must NOT be created on the foundation-exists block"
    );
}

#[test]
fn force_overwrite_regenerates_draft_from_scratch_with_warning() {
    // @step Given a project root tempdir that already has a spec/foundation.json.draft with custom content
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        "{\"version\":\"2.0.0\",\"project\":{\"name\":\"custom\"}}",
    );

    // @step When I dispatch discover-foundation with force=true
    let result = dispatch_command(req(tmp.path(), json!({"force": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the draft on disk contains the placeholder "[QUESTION: What is the project name?]"
    let draft = read_to_string(&tmp.path().join("spec/foundation.json.draft"));
    assert!(
        draft.contains("[QUESTION: What is the project name?]"),
        "draft regenerated: {draft}"
    );

    // @step And the returned systemReminder contains a force-overwrite warning
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("overwritten with --force"),
        "reminder: {reminder}"
    );

    // @step And forceOverwriteWarning is true because a draft was actually overwritten
    assert_eq!(
        data["forceOverwriteWarning"].as_bool(),
        Some(true),
        "force over an existing draft must set the stderr warning flag: {data}"
    );
}

#[test]
fn force_without_existing_draft_still_shows_stdout_banner_but_no_stderr_warning() {
    // PARITY (discover-foundation.ts:669-679 + 735-740): the STDOUT banner is
    // gated on `options.force` ALONE (so it shows even with no prior draft),
    // while the STDERR `output.warn` only fires when a draft was actually
    // overwritten. Here no draft exists, so the banner shows but the stderr
    // warning flag is false.

    // @step Given an empty project root tempdir with no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch discover-foundation with force=true
    let result = dispatch_command(req(tmp.path(), json!({"force": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the returned systemReminder still contains the force-overwrite banner
    let reminder = data["systemReminder"].as_str().unwrap_or_default();
    assert!(
        reminder.contains("WARNING: Existing draft was overwritten with --force flag"),
        "banner must show on --force even without a prior draft: {reminder}"
    );

    // @step And forceOverwriteWarning is false because no draft was actually overwritten
    assert_eq!(
        data["forceOverwriteWarning"].as_bool(),
        Some(false),
        "no prior draft → no stderr warning: {data}"
    );
}

#[test]
fn finalize_blocked_when_draft_still_has_placeholder_fields() {
    // @step Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&placeholder_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned validationErrors contains "Cannot finalize: draft still has unfilled placeholder fields"
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.contains("Cannot finalize: draft still has unfilled placeholder fields"),
        "validationErrors: {errors}"
    );

    // @step And spec/foundation.json is not created
    assert!(
        !tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must not be written"
    );

    // @step And spec/foundation.json.draft still exists on disk
    assert!(
        tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must remain"
    );
}

#[test]
fn finalize_blocked_when_filled_draft_violates_schema() {
    // @step Given a project root tempdir whose spec/foundation.json.draft has no placeholders but empty solutionSpace.capabilities
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = valid_filled_draft();
    draft["solutionSpace"]["capabilities"] = json!([]);
    let body = serde_json::to_string_pretty(&draft).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    // @step And the returned validationErrors starts with "Schema validation failed."
    let errors = data["validationErrors"].as_str().unwrap_or_default();
    assert!(
        errors.starts_with("Schema validation failed."),
        "validationErrors: {errors}"
    );

    // @step And spec/foundation.json is not created
    assert!(
        !tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must not be written"
    );
}

#[test]
fn finalize_success_writes_foundation_deletes_draft_and_creates_found_unit() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);

    // @step When I dispatch discover-foundation with finalize=true and autoGenerateMd=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"finalize": true, "autoGenerateMd": true}),
    ));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And the returned completionMessage contains "Discovery complete!"
    let msg = data["completionMessage"].as_str().unwrap_or_default();
    assert!(
        msg.contains("Discovery complete!"),
        "completionMessage: {msg}"
    );

    // @step And spec/foundation.json exists on disk
    assert!(
        tmp.path().join("spec/foundation.json").exists(),
        "foundation.json must be written"
    );

    // @step And spec/foundation.json.draft no longer exists on disk
    assert!(
        !tmp.path().join("spec/foundation.json.draft").exists(),
        "draft must be deleted after finalize"
    );

    // @step And spec/FOUNDATION.md exists on disk
    assert!(
        tmp.path().join("spec/FOUNDATION.md").exists(),
        "FOUNDATION.md must be generated"
    );

    // @step And spec/work-units.json contains a FOUND-prefixed work unit
    let wu = read_to_string(&tmp.path().join("spec/work-units.json"));
    assert!(
        wu.contains("FOUND-"),
        "work-units.json must contain a FOUND- unit: {wu}"
    );

    // @step And the envelope reports workUnitCreated=true with the FOUND-001 id (parity with the TS CLI "Created work unit" lines)
    assert_eq!(
        data["workUnitCreated"].as_bool(),
        Some(true),
        "workUnitCreated: {data}"
    );
    assert_eq!(
        data["workUnitId"].as_str(),
        Some("FOUND-001"),
        "workUnitId: {data}"
    );

    // @step And the FOUND task matches the TS createWorkUnit shape (stateHistory present, no children array, no prefixCounters)
    let wu_json: Value = serde_json::from_str(&wu).expect("parse work-units.json");
    let task = &wu_json["workUnits"]["FOUND-001"];
    assert_eq!(
        task["type"].as_str(),
        Some("task"),
        "FOUND task type: {task}"
    );
    assert!(
        task.get("stateHistory").and_then(Value::as_array).is_some(),
        "FOUND task must carry stateHistory: {task}"
    );
    assert_eq!(
        task["stateHistory"][0]["state"].as_str(),
        Some("backlog"),
        "stateHistory[0].state: {task}"
    );
    assert!(
        task.get("children").is_none(),
        "FOUND task must NOT have a children array: {task}"
    );
    assert!(
        wu_json.get("prefixCounters").is_none(),
        "work-units.json must NOT carry prefixCounters (TS createWorkUnit never writes it): {wu_json}"
    );
}

#[test]
fn finalize_found_autocreation_is_idempotent_when_found_unit_already_exists() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and a FOUND-001 work unit already exists
    let tmp = TempDir::new().expect("tempdir");
    let body = serde_json::to_string_pretty(&valid_filled_draft()).unwrap();
    write_draft_raw(tmp.path(), &body);
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let seed = json!({
        "workUnits": {
            "FOUND-001": {
                "id": "FOUND-001",
                "title": "Existing Foundation Event Storm",
                "type": "task",
                "status": "backlog",
                "createdAt": "x",
                "updatedAt": "x"
            }
        },
        "states": { "backlog": ["FOUND-001"], "specifying": [], "testing": [],
                    "implementing": [], "validating": [], "done": [], "blocked": [] },
        "prefixCounters": { "FOUND": 1 }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&seed).unwrap(),
    )
    .expect("write work-units.json");

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(tmp.path(), json!({"finalize": true})));

    // @step Then the dispatcher returns valid=true
    assert!(result.success, "expected success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(true),
        "valid must be true: {data}"
    );

    // @step And spec/work-units.json still has exactly one FOUND-prefixed work unit
    let wu: Value = serde_json::from_str(&read_to_string(&spec.join("work-units.json"))).unwrap();
    let found_count = wu["workUnits"]
        .as_object()
        .map(|m| m.keys().filter(|k| k.starts_with("FOUND-")).count())
        .unwrap_or(0);
    assert_eq!(
        found_count, 1,
        "expected exactly one FOUND- unit; got {found_count}"
    );

    // @step And the envelope reports workUnitCreated=false reusing the existing FOUND-001 id
    assert_eq!(
        data["workUnitCreated"].as_bool(),
        Some(false),
        "workUnitCreated: {data}"
    );
    assert_eq!(
        data["workUnitId"].as_str(),
        Some("FOUND-001"),
        "workUnitId: {data}"
    );
}
