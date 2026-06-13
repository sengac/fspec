#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-example-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-example`
// (RPC-181). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-example".to_string(),
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
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// Seed a minimal work-units.json containing one unit. `extra_fields` is
/// merged on top of the canonical id/title/type/status/createdAt/updatedAt
/// shape; pass `serde_json::Map::new()` for the default.
fn seed_one(id: &str, status: &str, extra_fields: serde_json::Map<String, Value>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), Value::String(id.to_string()));
    wu.insert("title".into(), Value::String(format!("title {id}")));
    wu.insert("type".into(), Value::String("story".into()));
    wu.insert("status".into(), Value::String(status.to_string()));
    wu.insert(
        "createdAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    wu.insert(
        "updatedAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    for (k, v) in extra_fields {
        wu.insert(k, v);
    }
    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(wu));
    let mut state_arrays = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        state_arrays.insert((*st).to_string(), Value::Array(arr));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(state_arrays),
    }))
    .unwrap()
}

// ---------- scenarios ----------

#[test]
fn first_example_appended_with_stable_id_zero_and_next_example_id_bumped_to_one() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no examples array
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", serde_json::Map::new());
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='User logs in with valid credentials'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "User logs in with valid credentials"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].id=0
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["examples"][0]["id"].as_u64(),
        Some(0)
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].text='User logs in with valid credentials'
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["examples"][0]["text"],
        "User logs in with valid credentials"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["examples"][0]["deleted"],
        false
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].createdAt is a freshly bumped ISO-8601 timestamp
    let created_at = on_disk["workUnits"]["AUTH-001"]["examples"][0]["createdAt"]
        .as_str()
        .expect("createdAt str");
    assert!(created_at.ends_with('Z') && created_at.contains('T'));
    assert_ne!(created_at, "2026-06-01T00:00:00.000Z");

    // @step And spec/work-units.json on disk shows AUTH-001.nextExampleId=1
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["nextExampleId"].as_u64(),
        Some(1)
    );

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated_at = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt str");
    assert_ne!(updated_at, "2026-06-01T00:00:00.000Z");
}

#[test]
fn second_example_reuses_the_incrementing_counter() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with one existing example id=0 and nextExampleId=1
    let tmp = TempDir::new().expect("tempdir");
    let mut extra = serde_json::Map::new();
    extra.insert(
        "examples".to_string(),
        json!([{
            "id": 0,
            "text": "First",
            "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }]),
    );
    extra.insert("nextExampleId".to_string(), json!(1));
    let pre = seed_one("AUTH-001", "specifying", extra);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='User enters wrong password'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "User enters wrong password"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.examples has length 2
    let examples = on_disk["workUnits"]["AUTH-001"]["examples"]
        .as_array()
        .expect("examples array");
    assert_eq!(examples.len(), 2);

    // @step And spec/work-units.json on disk shows AUTH-001.examples[1].id=1
    assert_eq!(examples[1]["id"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.examples[1].text='User enters wrong password'
    assert_eq!(examples[1]["text"], "User enters wrong password");

    // @step And spec/work-units.json on disk shows AUTH-001.nextExampleId=2
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["nextExampleId"].as_u64(),
        Some(2)
    );
}

#[test]
fn status_guard_rejects_add_example_when_work_unit_is_not_in_specifying() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "backlog", serde_json::Map::new());
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "Anything"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error message");

    // @step And the error message contains the substring "Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        err_msg.contains("Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "unexpected error: {err_msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn missing_work_unit_surfaces_the_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", serde_json::Map::new());
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-example with workUnitId='NOPE-001' and example='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "example": "Anything"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error message");

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        err_msg.contains("Work unit 'NOPE-001' does not exist"),
        "unexpected error: {err_msg}"
    );
}

#[test]
fn auto_creates_spec_work_units_json_then_reports_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "Anything"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error message");

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    assert!(
        err_msg.contains("Work unit 'AUTH-001' does not exist"),
        "unexpected error: {err_msg}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    let path = tmp.path().join("spec").join("work-units.json");
    assert!(path.exists(), "spec/work-units.json must be auto-created");
    let on_disk = read_work_units(tmp.path());
    assert_eq!(on_disk["version"], "0.7.1");
    assert!(on_disk["workUnits"]
        .as_object()
        .expect("workUnits object")
        .is_empty());
}

#[test]
fn success_rendering_embeds_the_system_reminder_block() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with userStory.role='developer'
    let tmp = TempDir::new().expect("tempdir");
    let mut extra = serde_json::Map::new();
    extra.insert(
        "userStory".to_string(),
        json!({"role": "developer", "action": "test", "benefit": "win"}),
    );
    let pre = seed_one("AUTH-001", "specifying", extra);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='Valid login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "Valid login"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output starts with "✓ Example added successfully"
    assert!(
        result.data.starts_with("✓ Example added successfully"),
        "got: {}",
        result.data
    );

    // @step And the rendered output contains the substring "<system-reminder>"
    assert!(
        result.data.contains("<system-reminder>"),
        "got: {}",
        result.data
    );

    // @step And the rendered output contains the substring "User story: \"As a developer...\""
    assert!(
        result.data.contains("User story: \"As a developer...\""),
        "got: {}",
        result.data
    );

    // @step And the rendered output contains the substring "Example: \"Valid login\""
    assert!(
        result.data.contains("Example: \"Valid login\""),
        "got: {}",
        result.data
    );

    // @step And the rendered output contains the substring "</system-reminder>"
    assert!(
        result.data.contains("</system-reminder>"),
        "got: {}",
        result.data
    );
}

#[test]
fn system_reminder_falls_back_to_the_user_when_user_story_role_is_absent() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no userStory
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", serde_json::Map::new());
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-example with workUnitId='AUTH-001' and example='Valid login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "example": "Valid login"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring "User story: \"As a the user...\""
    assert!(
        result.data.contains("User story: \"As a the user...\""),
        "got: {}",
        result.data
    );
}
