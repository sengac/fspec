#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-dependency-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-dependency` (RPC-177).
// Each #[test] fn maps to one Gherkin scenario; @step comments mirror the Gherkin
// step text verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-dependency".to_string(),
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

/// Seed a work-units.json with the given (id, status) pairs.
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

// ---------- scenarios ----------

#[test]
fn depends_on_shorthand_seeds_depends_on_array_on_source() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and AUTH-002 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying"), ("AUTH-002", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='AUTH-002' and dependsOn='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-002", "dependsOn": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-002.dependsOn=['AUTH-001']
    let v = read_work_units(tmp.path());
    let deps = v["workUnits"]["AUTH-002"]["dependsOn"].as_array().expect("dependsOn array");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].as_str(), Some("AUTH-001"));

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks or blockedBy edge added
    assert!(v["workUnits"]["AUTH-001"]["blocks"].as_array().map_or(true, |a| a.is_empty()));
    assert!(v["workUnits"]["AUTH-001"]["blockedBy"].as_array().map_or(true, |a| a.is_empty()));

    // @step And spec/work-units.json on disk shows AUTH-002.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-002"]["updatedAt"].as_str().expect("updatedAt");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"));
}

#[test]
fn blocks_creates_bidirectional_edge_and_auto_transitions_target_to_blocked() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and API-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying"), ("API-001", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='API-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "blocks": "API-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.blocks=['API-001']
    let blocks = v["workUnits"]["AUTH-001"]["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].as_str(), Some("API-001"));

    // @step And spec/work-units.json on disk shows API-001.blockedBy=['AUTH-001']
    let bb = v["workUnits"]["API-001"]["blockedBy"].as_array().expect("blockedBy array");
    assert_eq!(bb.len(), 1);
    assert_eq!(bb[0].as_str(), Some("AUTH-001"));

    // @step And spec/work-units.json on disk shows API-001.status='blocked'
    assert_eq!(v["workUnits"]["API-001"]["status"].as_str(), Some("blocked"));

    // @step And spec/work-units.json on disk shows states.specifying no longer contains 'API-001'
    let spec_states = v["states"]["specifying"].as_array().expect("states.specifying");
    assert!(!spec_states.iter().any(|x| x.as_str() == Some("API-001")));

    // @step And spec/work-units.json on disk shows states.blocked contains 'API-001'
    let blocked_states = v["states"]["blocked"].as_array().expect("states.blocked");
    assert!(blocked_states.iter().any(|x| x.as_str() == Some("API-001")));
}

#[test]
fn blocks_targeting_done_unit_does_not_transition_its_status() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and API-001 status=done
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying"), ("API-001", "done")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='API-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "blocks": "API-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows API-001.status='done'
    assert_eq!(v["workUnits"]["API-001"]["status"].as_str(), Some("done"));
    // @step And spec/work-units.json on disk shows states.done still contains 'API-001'
    let done_states = v["states"]["done"].as_array().expect("states.done");
    assert!(done_states.iter().any(|x| x.as_str() == Some("API-001")));
    // @step And spec/work-units.json on disk shows states.blocked does not contain 'API-001'
    let blocked_states = v["states"]["blocked"].as_array().expect("states.blocked");
    assert!(!blocked_states.iter().any(|x| x.as_str() == Some("API-001")));
}

#[test]
fn blocked_by_creates_bidirectional_edge_and_auto_transitions_source_with_blocked_reason() {
    // @step Given a project root tempdir with spec/work-units.json containing UI-001 status=specifying and API-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("UI-001", "specifying"), ("API-001", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='UI-001' and blockedBy='API-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UI-001", "blockedBy": "API-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows UI-001.blockedBy=['API-001']
    let bb = v["workUnits"]["UI-001"]["blockedBy"].as_array().expect("blockedBy array");
    assert_eq!(bb.len(), 1);
    assert_eq!(bb[0].as_str(), Some("API-001"));

    // @step And spec/work-units.json on disk shows API-001.blocks=['UI-001']
    let bl = v["workUnits"]["API-001"]["blocks"].as_array().expect("blocks array");
    assert_eq!(bl.len(), 1);
    assert_eq!(bl[0].as_str(), Some("UI-001"));

    // @step And spec/work-units.json on disk shows UI-001.status='blocked'
    assert_eq!(v["workUnits"]["UI-001"]["status"].as_str(), Some("blocked"));

    // @step And spec/work-units.json on disk shows UI-001.blockedReason='Blocked by API-001'
    assert_eq!(v["workUnits"]["UI-001"]["blockedReason"].as_str(), Some("Blocked by API-001"));

    // @step And spec/work-units.json on disk shows states.specifying no longer contains 'UI-001'
    let spec_states = v["states"]["specifying"].as_array().expect("states.specifying");
    assert!(!spec_states.iter().any(|x| x.as_str() == Some("UI-001")));

    // @step And spec/work-units.json on disk shows states.blocked contains 'UI-001'
    let blocked_states = v["states"]["blocked"].as_array().expect("states.blocked");
    assert!(blocked_states.iter().any(|x| x.as_str() == Some("UI-001")));
}

#[test]
fn depends_on_flag_creates_unidirectional_edge_only() {
    // @step Given a project root tempdir with spec/work-units.json containing DASH-001 status=specifying and AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("DASH-001", "specifying"), ("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='DASH-001' and dependsOn='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "DASH-001", "dependsOn": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows DASH-001.dependsOn=['AUTH-001']
    let deps = v["workUnits"]["DASH-001"]["dependsOn"].as_array().expect("dependsOn array");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].as_str(), Some("AUTH-001"));

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks edge added
    assert!(v["workUnits"]["AUTH-001"]["blocks"].as_array().map_or(true, |a| a.is_empty()));

    // @step And spec/work-units.json on disk shows AUTH-001.status remains unchanged
    assert_eq!(v["workUnits"]["AUTH-001"]["status"].as_str(), Some("specifying"));
}

#[test]
fn relates_to_creates_symmetric_edges() {
    // @step Given a project root tempdir with spec/work-units.json containing UI-005 status=specifying and UI-004 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("UI-005", "specifying"), ("UI-004", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-dependency with workUnitId='UI-005' and relatesTo='UI-004'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UI-005", "relatesTo": "UI-004"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows UI-005.relatesTo=['UI-004']
    let r5 = v["workUnits"]["UI-005"]["relatesTo"].as_array().expect("UI-005 relatesTo");
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].as_str(), Some("UI-004"));

    // @step And spec/work-units.json on disk shows UI-004.relatesTo=['UI-005']
    let r4 = v["workUnits"]["UI-004"]["relatesTo"].as_array().expect("UI-004 relatesTo");
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].as_str(), Some("UI-005"));

    // @step And spec/work-units.json on disk shows neither UI-005 nor UI-004 changed status
    assert_eq!(v["workUnits"]["UI-005"]["status"].as_str(), Some("specifying"));
    assert_eq!(v["workUnits"]["UI-004"]["status"].as_str(), Some("specifying"));
}

#[test]
fn relates_to_reverse_edge_is_idempotent_when_already_present() {
    // @step Given a project root tempdir with spec/work-units.json containing UI-005 status=specifying and UI-004 status=specifying with UI-004.relatesTo already containing 'UI-005'
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_units(&[("UI-005", "specifying"), ("UI-004", "specifying")])).unwrap();
    pre["workUnits"]["UI-004"]["relatesTo"] = json!(["UI-005"]);
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch add-dependency with workUnitId='UI-005' and relatesTo='UI-004'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "UI-005", "relatesTo": "UI-004"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows UI-005.relatesTo=['UI-004']
    let r5 = v["workUnits"]["UI-005"]["relatesTo"].as_array().expect("UI-005 relatesTo");
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].as_str(), Some("UI-004"));

    // @step And spec/work-units.json on disk shows UI-004.relatesTo=['UI-005']
    let r4 = v["workUnits"]["UI-004"]["relatesTo"].as_array().expect("UI-004 relatesTo");
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].as_str(), Some("UI-005"));
}

#[test]
fn missing_source_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-dependency with workUnitId='NOPE-001' and dependsOn='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "dependsOn": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "expected canonical missing message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn missing_target_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='MISS-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependsOn": "MISS-001"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Target work unit 'MISS-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Target work unit 'MISS-001' does not exist"),
        "expected canonical missing-target message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn self_dependency_is_rejected_for_every_flag() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and blocks='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "blocks": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Cannot create self-dependency'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Cannot create self-dependency"),
        "expected canonical self-dep message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn duplicate_edge_is_rejected() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with dependsOn=['AUTH-000'] and AUTH-000 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_units(&[("AUTH-001", "specifying"), ("AUTH-000", "specifying")])).unwrap();
    pre["workUnits"]["AUTH-001"]["dependsOn"] = json!(["AUTH-000"]);
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='AUTH-000'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependsOn": "AUTH-000"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Dependency already exists'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Dependency already exists"),
        "expected duplicate-edge message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn circular_blocks_chain_is_rejected_before_disk_write() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with blocks=['AUTH-002'] and AUTH-002 status=blocked with blockedBy=['AUTH-001']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_units(&[("AUTH-001", "specifying"), ("AUTH-002", "blocked")])).unwrap();
    pre["workUnits"]["AUTH-001"]["blocks"] = json!(["AUTH-002"]);
    pre["workUnits"]["AUTH-002"]["blockedBy"] = json!(["AUTH-001"]);
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-dependency with workUnitId='AUTH-002' and blocks='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-002", "blocks": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Circular dependency detected: AUTH-002 -> '
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Circular dependency detected: AUTH-002 -> "),
        "expected canonical cycle message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn auto_creates_work_units_then_reports_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-dependency with workUnitId='AUTH-001' and dependsOn='AUTH-000'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "dependsOn": "AUTH-000"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'AUTH-001' does not exist"),
        "expected canonical missing source; got: {err}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    assert!(tmp.path().join("spec/work-units.json").exists());
    let v = read_work_units(tmp.path());
    assert_eq!(v["version"].as_str(), Some("0.7.1"));
    assert!(v["workUnits"].as_object().unwrap().is_empty());
    assert!(v["states"]["backlog"].as_array().unwrap().is_empty());
}
