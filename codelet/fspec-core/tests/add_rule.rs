#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-rule-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-rule` (RPC-189).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-rule".to_string(),
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
fn first_add_seeds_rules_array_and_next_rule_id_on_clean_specifying_unit() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no rules field
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch add-rule with workUnitId='AUTH-001' and rule='Email must be valid format'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "rule": "Email must be valid format"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains ruleCount=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["ruleCount"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].id=0
    let v = read_work_units(tmp.path());
    let r0 = &v["workUnits"]["AUTH-001"]["rules"][0];
    assert_eq!(r0["id"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].text='Email must be valid format'
    assert_eq!(r0["text"].as_str(), Some("Email must be valid format"));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    assert_eq!(r0["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].createdAt is a freshly bumped ISO-8601 timestamp
    let created = r0["createdAt"].as_str().expect("createdAt string");
    assert!(created.len() == 24 && created.ends_with('Z'), "got: {created}");
    assert!(!created.starts_with("2026-06-01"), "createdAt must NOT be the seed value");

    // @step And spec/work-units.json on disk shows AUTH-001.nextRuleId=1
    assert_eq!(v["workUnits"]["AUTH-001"]["nextRuleId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt string");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"), "updatedAt must NOT be the seed value");
}

#[test]
fn second_add_appends_with_auto_incremented_id() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with rules=[{id:0,text:'r1',deleted:false,createdAt:'x'}] and nextRuleId=1
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_units(&[("AUTH-001", "specifying")])).unwrap();
    pre["workUnits"]["AUTH-001"]["rules"] = json!([{
        "id": 0,
        "text": "r1",
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    }]);
    pre["workUnits"]["AUTH-001"]["nextRuleId"] = json!(1);
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch add-rule with workUnitId='AUTH-001' and rule='Password must be 8+ chars'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "rule": "Password must be 8+ chars"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains ruleCount=2
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["ruleCount"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk shows AUTH-001.rules has length 2
    let v = read_work_units(tmp.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"].as_array().expect("rules array");
    assert_eq!(rules.len(), 2);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].id=1
    assert_eq!(rules[1]["id"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].text='Password must be 8+ chars'
    assert_eq!(rules[1]["text"].as_str(), Some("Password must be 8+ chars"));

    // @step And spec/work-units.json on disk shows AUTH-001.nextRuleId=2
    assert_eq!(v["workUnits"]["AUTH-001"]["nextRuleId"].as_u64(), Some(2));
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "specifying")]);
    write_work_units(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-rule with workUnitId='NOPE-001' and rule='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "rule": "Anything"}),
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
    assert_eq!(pre_bytes, post_bytes, "work-units.json must NOT be mutated on failure");
}

#[test]
fn non_specifying_status_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_units(&[("AUTH-001", "backlog")]);
    write_work_units(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-rule with workUnitId='AUTH-001' and rule='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "rule": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "expected canonical phase-guard message; got: {err}"
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

    // @step When I dispatch add-rule with workUnitId='AUTH-001' and rule='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "rule": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'AUTH-001' does not exist"),
        "expected canonical missing message; got: {err}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    assert!(tmp.path().join("spec/work-units.json").exists());
    let v = read_work_units(tmp.path());
    assert_eq!(v["version"].as_str(), Some("0.7.1"));
    assert!(v["workUnits"].as_object().unwrap().is_empty());
    assert!(v["states"]["backlog"].as_array().unwrap().is_empty());
}
