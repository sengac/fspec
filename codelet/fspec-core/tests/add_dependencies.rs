#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-dependencies-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-dependencies`
// (RPC-176). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-dependencies".to_string(),
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

/// Seed a work-units.json with the given (id, status) pairs. Initialises
/// the canonical 7 state arrays and inserts each id under its status bucket.
fn seed_units(units: &[(&str, &str)]) -> String {
    let mut wus = serde_json::Map::new();
    let mut states: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert(
            "createdAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        obj.insert(
            "updatedAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        wus.insert((*id).to_string(), Value::Object(obj));
        states
            .get_mut(*status)
            .expect("known state")
            .push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(
                states
                    .get(*st)
                    .expect("seeded state")
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states_obj),
    }))
    .unwrap()
}

/// Override a work-unit's dependency array on disk (for cycle-test seeding).
fn add_blocks_to_unit(json_str: &str, id: &str, blocks: &[&str]) -> String {
    let mut v: Value = serde_json::from_str(json_str).unwrap();
    let arr: Vec<Value> = blocks
        .iter()
        .map(|s| Value::String((*s).to_string()))
        .collect();
    v["workUnits"][id]["blocks"] = Value::Array(arr);
    serde_json::to_string_pretty(&v).unwrap()
}

// ---------- scenarios ----------

#[test]
fn bulk_blocks_adds_bidirectional_edges_and_auto_transitions_targets_to_blocked() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001, AUTH-002, AUTH-003 all status=backlog with empty dependency arrays
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[
        ("AUTH-001", "backlog"),
        ("AUTH-002", "backlog"),
        ("AUTH-003", "backlog"),
    ]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002', 'AUTH-003']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependencies": {"blocks": ["AUTH-002", "AUTH-003"]}}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains added=2
    let data: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(data["added"].as_u64(), Some(2), "data: {}", result.data);

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.blocks=['AUTH-002', 'AUTH-003']
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["blocks"],
        json!(["AUTH-002", "AUTH-003"])
    );

    // @step And spec/work-units.json on disk shows AUTH-002.blockedBy contains 'AUTH-001'
    let bb2 = on_disk["workUnits"]["AUTH-002"]["blockedBy"]
        .as_array()
        .expect("blockedBy array");
    assert!(bb2.iter().any(|v| v == "AUTH-001"));

    // @step And spec/work-units.json on disk shows AUTH-002.status='blocked'
    assert_eq!(on_disk["workUnits"]["AUTH-002"]["status"], "blocked");

    // @step And spec/work-units.json on disk shows AUTH-003.blockedBy contains 'AUTH-001'
    let bb3 = on_disk["workUnits"]["AUTH-003"]["blockedBy"]
        .as_array()
        .expect("blockedBy array");
    assert!(bb3.iter().any(|v| v == "AUTH-001"));

    // @step And spec/work-units.json on disk shows AUTH-003.status='blocked'
    assert_eq!(on_disk["workUnits"]["AUTH-003"]["status"], "blocked");

    // @step And spec/work-units.json on disk shows states.backlog does NOT contain 'AUTH-002' or 'AUTH-003'
    let backlog = on_disk["states"]["backlog"].as_array().expect("backlog");
    assert!(!backlog.iter().any(|v| v == "AUTH-002"));
    assert!(!backlog.iter().any(|v| v == "AUTH-003"));

    // @step And spec/work-units.json on disk shows states.blocked contains 'AUTH-002' and 'AUTH-003'
    let blocked = on_disk["states"]["blocked"].as_array().expect("blocked");
    assert!(blocked.iter().any(|v| v == "AUTH-002"));
    assert!(blocked.iter().any(|v| v == "AUTH-003"));

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt str");
    assert_ne!(
        updated, "2026-06-01T00:00:00.000Z",
        "updatedAt must be bumped"
    );
    assert!(
        updated.ends_with('Z') && updated.contains('T'),
        "updatedAt must be ISO-8601: {updated}"
    );
}

#[test]
fn bulk_blocked_by_auto_transitions_source_to_blocked_with_blocked_reason() {
    // @step Given a project root tempdir with spec/work-units.json containing UI-001 status=specifying and API-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("UI-001", "specifying"), ("API-001", "backlog")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependencies with workUnitId='UI-001' and dependencies.blockedBy=['API-001']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UI-001", "dependencies": {"blockedBy": ["API-001"]}}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains added=1
    let data: Value = serde_json::from_str(&result.data).expect("JSON");
    assert_eq!(data["added"].as_u64(), Some(1));

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows UI-001.blockedBy=['API-001']
    assert_eq!(
        on_disk["workUnits"]["UI-001"]["blockedBy"],
        json!(["API-001"])
    );

    // @step And spec/work-units.json on disk shows API-001.blocks contains 'UI-001'
    let blocks = on_disk["workUnits"]["API-001"]["blocks"]
        .as_array()
        .expect("blocks");
    assert!(blocks.iter().any(|v| v == "UI-001"));

    // @step And spec/work-units.json on disk shows UI-001.status='blocked'
    assert_eq!(on_disk["workUnits"]["UI-001"]["status"], "blocked");

    // @step And spec/work-units.json on disk shows UI-001.blockedReason='Blocked by API-001'
    assert_eq!(
        on_disk["workUnits"]["UI-001"]["blockedReason"],
        "Blocked by API-001"
    );

    // @step And spec/work-units.json on disk shows states.specifying does NOT contain 'UI-001'
    let specifying = on_disk["states"]["specifying"]
        .as_array()
        .expect("specifying");
    assert!(!specifying.iter().any(|v| v == "UI-001"));

    // @step And spec/work-units.json on disk shows states.blocked contains 'UI-001'
    let blocked = on_disk["states"]["blocked"].as_array().expect("blocked");
    assert!(blocked.iter().any(|v| v == "UI-001"));
}

#[test]
fn bulk_depends_on_writes_only_source_array_with_no_reverse_or_status_change() {
    // @step Given a project root tempdir with spec/work-units.json containing DASH-001 status=backlog, AUTH-001 status=backlog, AUTH-002 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[
        ("DASH-001", "backlog"),
        ("AUTH-001", "backlog"),
        ("AUTH-002", "backlog"),
    ]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependencies with workUnitId='DASH-001' and dependencies.dependsOn=['AUTH-001', 'AUTH-002']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DASH-001", "dependencies": {"dependsOn": ["AUTH-001", "AUTH-002"]}}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    // @step And the returned data contains added=2
    let data: Value = serde_json::from_str(&result.data).unwrap();
    assert_eq!(data["added"].as_u64(), Some(2));

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows DASH-001.dependsOn=['AUTH-001', 'AUTH-002']
    assert_eq!(
        on_disk["workUnits"]["DASH-001"]["dependsOn"],
        json!(["AUTH-001", "AUTH-002"])
    );

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks, no blockedBy, no dependsOn, no relatesTo fields
    for field in &["blocks", "blockedBy", "dependsOn", "relatesTo"] {
        assert!(
            on_disk["workUnits"]["AUTH-001"].get(*field).is_none(),
            "AUTH-001.{field} should be absent"
        );
        assert!(
            on_disk["workUnits"]["AUTH-002"].get(*field).is_none(),
            "AUTH-002.{field} should be absent"
        );
    }

    // @step And spec/work-units.json on disk shows AUTH-001.status='backlog' and AUTH-002.status='backlog'
    assert_eq!(on_disk["workUnits"]["AUTH-001"]["status"], "backlog");
    assert_eq!(on_disk["workUnits"]["AUTH-002"]["status"], "backlog");
}

#[test]
fn bulk_relates_to_creates_symmetric_edge_on_both_sides() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-002 and AUTH-003 both status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-002", "backlog"), ("AUTH-003", "backlog")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependencies with workUnitId='AUTH-002' and dependencies.relatesTo=['AUTH-003']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-002", "dependencies": {"relatesTo": ["AUTH-003"]}}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    // @step And the returned data contains added=1
    let data: Value = serde_json::from_str(&result.data).unwrap();
    assert_eq!(data["added"].as_u64(), Some(1));

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-002.relatesTo=['AUTH-003']
    assert_eq!(
        on_disk["workUnits"]["AUTH-002"]["relatesTo"],
        json!(["AUTH-003"])
    );

    // @step And spec/work-units.json on disk shows AUTH-003.relatesTo=['AUTH-002']
    assert_eq!(
        on_disk["workUnits"]["AUTH-003"]["relatesTo"],
        json!(["AUTH-002"])
    );

    // @step And spec/work-units.json on disk shows AUTH-002.status='backlog' and AUTH-003.status='backlog'
    assert_eq!(on_disk["workUnits"]["AUTH-002"]["status"], "backlog");
    assert_eq!(on_disk["workUnits"]["AUTH-003"]["status"], "backlog");
}

#[test]
fn missing_target_id_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog and no NOPE-999 work unit
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);
    let pre_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();

    // @step When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['NOPE-999']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependencies": {"blocks": ["NOPE-999"]}}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains "Target work unit 'NOPE-999' does not exist"
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Target work unit 'NOPE-999' does not exist"),
        "err was: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();
    assert_eq!(pre_disk, post_disk, "file must be byte-equal pre/post");
}

#[test]
fn self_dependency_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);
    let pre_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();

    // @step When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-001']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependencies": {"blocks": ["AUTH-001"]}}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Cannot create self-dependency'
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Cannot create self-dependency"),
        "err was: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();
    assert_eq!(pre_disk, post_disk);
}

#[test]
fn circular_blocks_chain_is_detected_and_rejected() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-002 already has blocks=['AUTH-001']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("AUTH-001", "backlog"), ("AUTH-002", "backlog")]);
    pre = add_blocks_to_unit(&pre, "AUTH-002", &["AUTH-001"]);
    write_work_units(tmp.path(), &pre);
    let pre_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();

    // @step When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependencies": {"blocks": ["AUTH-002"]}}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Circular dependency detected: AUTH-001 -> AUTH-002 -> AUTH-001'
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Circular dependency detected: AUTH-001 -> AUTH-002 -> AUTH-001"),
        "err was: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();
    assert_eq!(pre_disk, post_disk);
}

#[test]
fn missing_source_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependencies with workUnitId='NOPE-001' and dependencies.blocks=['AUTH-001']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "dependencies": {"blocks": ["AUTH-001"]}}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains "Work unit 'NOPE-001' does not exist"
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "err was: {err}"
    );
}

#[test]
fn auto_creates_work_units_json_when_missing_then_reports_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-dependencies with workUnitId='AUTH-001' and dependencies.blocks=['AUTH-002']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependencies": {"blocks": ["AUTH-002"]}}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains "Work unit 'AUTH-001' does not exist"
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Work unit 'AUTH-001' does not exist"),
        "err was: {err}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    let path = tmp.path().join("spec").join("work-units.json");
    assert!(path.exists(), "spec/work-units.json must be auto-created");
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["version"].as_str(), Some("0.7.1"));
    assert!(on_disk["workUnits"]
        .as_object()
        .map(serde_json::Map::is_empty)
        .unwrap_or(false));
}
