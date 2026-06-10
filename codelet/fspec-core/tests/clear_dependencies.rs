#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/clear-dependencies-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `clear-dependencies`
// (RPC-204). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "clear-dependencies".to_string(),
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
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("2026-06-01T00:00:00.000Z".to_string()));
        obj.insert("updatedAt".into(), Value::String("2026-06-01T00:00:00.000Z".to_string()));
        wus.insert((*id).to_string(), Value::Object(obj));
        states.get_mut(*status).expect("known state").push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(
                states.get(*st).expect("seeded state").iter()
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

/// Set an array-typed dep field on a unit (e.g. blocks/blockedBy/dependsOn/relatesTo).
fn set_field(json_str: &str, id: &str, field: &str, values: &[&str]) -> String {
    let mut v: Value = serde_json::from_str(json_str).unwrap();
    let arr: Vec<Value> = values.iter().map(|s| Value::String((*s).to_string())).collect();
    v["workUnits"][id][field] = Value::Array(arr);
    serde_json::to_string_pretty(&v).unwrap()
}

// ---------- scenarios ----------

#[test]
fn missing_confirm_flag_fails_before_any_file_io() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with blocks=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("AUTH-001", "backlog"), ("AUTH-002", "backlog")]);
    pre = set_field(&pre, "AUTH-001", "blocks", &["AUTH-002"]);
    write_work_units(tmp.path(), &pre);
    let pre_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": false}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Must confirm clearing all dependencies with --confirm flag'
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Must confirm clearing all dependencies with --confirm flag"),
        "err was: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_disk = fs::read_to_string(tmp.path().join("spec").join("work-units.json")).unwrap();
    assert_eq!(pre_disk, post_disk, "file must be byte-equal pre/post");
}

#[test]
fn missing_source_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='UNKNOWN-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UNKNOWN-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains "Work unit 'UNKNOWN-001' does not exist"
    let err = result.error.as_deref().unwrap_or("");
    assert!(err.contains("Work unit 'UNKNOWN-001' does not exist"), "err was: {err}");
}

#[test]
fn mixed_blocks_and_depends_on_are_removed_with_bidirectional_cleanup() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 with blocks=['AUTH-002'] dependsOn=['API-001'], AUTH-002 with blockedBy=['AUTH-001'], and API-001
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[
        ("AUTH-001", "backlog"),
        ("AUTH-002", "backlog"),
        ("API-001", "backlog"),
    ]);
    pre = set_field(&pre, "AUTH-001", "blocks", &["AUTH-002"]);
    pre = set_field(&pre, "AUTH-001", "dependsOn", &["API-001"]);
    pre = set_field(&pre, "AUTH-002", "blockedBy", &["AUTH-001"]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks field and no dependsOn field
    assert!(on_disk["workUnits"]["AUTH-001"].get("blocks").is_none(), "AUTH-001.blocks should be absent");
    assert!(on_disk["workUnits"]["AUTH-001"].get("dependsOn").is_none(), "AUTH-001.dependsOn should be absent");

    // @step And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    assert!(on_disk["workUnits"]["AUTH-002"].get("blockedBy").is_none(), "AUTH-002.blockedBy should be absent");

    // @step And spec/work-units.json on disk shows API-001 has no blocks field and no blockedBy field
    assert!(on_disk["workUnits"]["API-001"].get("blocks").is_none(), "API-001.blocks should be absent");
    assert!(on_disk["workUnits"]["API-001"].get("blockedBy").is_none(), "API-001.blockedBy should be absent");

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"].as_str().expect("updatedAt str");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z", "updatedAt must be bumped");
    assert!(updated.ends_with('Z') && updated.contains('T'), "updatedAt must be ISO-8601: {updated}");
}

#[test]
fn relates_to_edges_are_symmetrically_dropped_from_both_sides() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.relatesTo=['UI-001', 'UI-002'], UI-001.relatesTo=['AUTH-001'], UI-002.relatesTo=['AUTH-001', 'OTHER-001']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[
        ("AUTH-001", "backlog"),
        ("UI-001", "backlog"),
        ("UI-002", "backlog"),
        ("OTHER-001", "backlog"),
    ]);
    pre = set_field(&pre, "AUTH-001", "relatesTo", &["UI-001", "UI-002"]);
    pre = set_field(&pre, "UI-001", "relatesTo", &["AUTH-001"]);
    pre = set_field(&pre, "UI-002", "relatesTo", &["AUTH-001", "OTHER-001"]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 has no relatesTo field
    assert!(on_disk["workUnits"]["AUTH-001"].get("relatesTo").is_none(), "AUTH-001.relatesTo should be absent");

    // @step And spec/work-units.json on disk shows UI-001 has no relatesTo field
    assert!(on_disk["workUnits"]["UI-001"].get("relatesTo").is_none(), "UI-001.relatesTo should be absent");

    // @step And spec/work-units.json on disk shows UI-002.relatesTo=['OTHER-001']
    assert_eq!(on_disk["workUnits"]["UI-002"]["relatesTo"], json!(["OTHER-001"]));
}

#[test]
fn clearing_never_changes_a_blocked_work_units_status_or_state_array() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 has status='blocked' blockedBy=['API-001'] and states.blocked=['AUTH-001']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("AUTH-001", "blocked"), ("API-001", "backlog")]);
    pre = set_field(&pre, "AUTH-001", "blockedBy", &["API-001"]);
    pre = set_field(&pre, "API-001", "blocks", &["AUTH-001"]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.status='blocked'
    assert_eq!(on_disk["workUnits"]["AUTH-001"]["status"], "blocked");

    // @step And spec/work-units.json on disk shows states.blocked still contains 'AUTH-001'
    let blocked = on_disk["states"]["blocked"].as_array().expect("blocked");
    assert!(blocked.iter().any(|v| v == "AUTH-001"), "states.blocked must still contain AUTH-001");

    // @step And spec/work-units.json on disk shows states.backlog does NOT contain 'AUTH-001'
    let backlog = on_disk["states"]["backlog"].as_array().expect("backlog");
    assert!(!backlog.iter().any(|v| v == "AUTH-001"), "states.backlog must not contain AUTH-001");
}

#[test]
fn reverse_edge_cleanup_is_silently_skipped_when_target_is_missing() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 has blocks=['GHOST-999'] and GHOST-999 does not exist as a work unit
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("AUTH-001", "backlog")]);
    pre = set_field(&pre, "AUTH-001", "blocks", &["GHOST-999"]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks field
    assert!(on_disk["workUnits"]["AUTH-001"].get("blocks").is_none(), "AUTH-001.blocks should be absent");

    // @step And no error is raised for the missing GHOST-999 work unit
    assert!(result.error.is_none(), "no error should be reported, got {:?}", result.error);
}

#[test]
fn no_dependency_arrays_still_succeeds_and_only_bumps_updated_at() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 with no blocks, blockedBy, dependsOn, or relatesTo fields
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='AUTH-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 still has no blocks, blockedBy, dependsOn, or relatesTo fields
    for field in &["blocks", "blockedBy", "dependsOn", "relatesTo"] {
        assert!(on_disk["workUnits"]["AUTH-001"].get(*field).is_none(), "AUTH-001.{field} should be absent");
    }

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"].as_str().expect("updatedAt str");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z", "updatedAt must be bumped");
    assert!(updated.ends_with('Z') && updated.contains('T'), "updatedAt must be ISO-8601: {updated}");
}

#[test]
fn blocked_by_clearing_reverse_removes_source_from_each_targets_blocks_array() {
    // @step Given a project root tempdir with spec/work-units.json where UI-001.blockedBy=['API-001', 'DB-001'], API-001.blocks=['UI-001'], DB-001.blocks=['UI-001', 'UI-002']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[
        ("UI-001", "backlog"),
        ("UI-002", "backlog"),
        ("API-001", "backlog"),
        ("DB-001", "backlog"),
    ]);
    pre = set_field(&pre, "UI-001", "blockedBy", &["API-001", "DB-001"]);
    pre = set_field(&pre, "API-001", "blocks", &["UI-001"]);
    pre = set_field(&pre, "DB-001", "blocks", &["UI-001", "UI-002"]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch clear-dependencies with workUnitId='UI-001' and confirm=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UI-001", "confirm": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows UI-001 has no blockedBy field
    assert!(on_disk["workUnits"]["UI-001"].get("blockedBy").is_none(), "UI-001.blockedBy should be absent");

    // @step And spec/work-units.json on disk shows API-001 has no blocks field
    assert!(on_disk["workUnits"]["API-001"].get("blocks").is_none(), "API-001.blocks should be absent");

    // @step And spec/work-units.json on disk shows DB-001.blocks=['UI-002']
    assert_eq!(on_disk["workUnits"]["DB-001"]["blocks"], json!(["UI-002"]));
}
