#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-story-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-story`
// (RPC-214). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "create-story".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn spec_dir(project_root: &Path) -> std::path::PathBuf {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    spec
}

/// Write a minimal valid spec/foundation.json so checkFoundationExists passes.
fn write_foundation(project_root: &Path) {
    let spec = spec_dir(project_root);
    fs::write(spec.join("foundation.json"), r#"{"version":"2.0.0"}"#)
        .expect("write foundation.json");
}

/// Write spec/prefixes.json registering the given prefixes.
fn write_prefixes(project_root: &Path, prefixes: &[&str]) {
    let spec = spec_dir(project_root);
    let mut obj = serde_json::Map::new();
    for p in prefixes {
        obj.insert(
            (*p).to_string(),
            json!({"prefix": p, "description": format!("{p} features"), "createdAt": "2026-06-01T00:00:00.000Z"}),
        );
    }
    let data = json!({"prefixes": Value::Object(obj)});
    fs::write(
        spec.join("prefixes.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write prefixes.json");
}

/// Write spec/epics.json with the given epic ids.
fn write_epics(project_root: &Path, epics: &[&str]) {
    let spec = spec_dir(project_root);
    let mut obj = serde_json::Map::new();
    for e in epics {
        obj.insert(
            (*e).to_string(),
            json!({"id": e, "title": format!("title {e}"), "createdAt": "2026-06-01T00:00:00.000Z"}),
        );
    }
    let data = json!({"epics": Value::Object(obj)});
    fs::write(
        spec.join("epics.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write epics.json");
}

/// Write a raw spec/work-units.json string.
fn write_work_units(project_root: &Path, raw: &str) {
    let spec = spec_dir(project_root);
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn read_work_units_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/work-units.json")).expect("read work-units.json")
}

fn read_epics(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/epics.json")).expect("read epics.json");
    serde_json::from_str(&raw).expect("parse epics.json")
}

/// Build a work-units.json with the given work units, each as
/// (id, status, parent-or-empty). Sets prefixCounters from the highest
/// suffix per prefix.
fn build_work_units(units: &[(&str, &str, &str)]) -> String {
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
    for (id, status, parent) in units {
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
        if !parent.is_empty() {
            obj.insert("parent".into(), Value::String((*parent).to_string()));
        } else {
            obj.insert("children".into(), Value::Array(vec![]));
        }
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
                    .expect("state")
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

// ---------- scenarios ----------

#[test]
fn dispatcher_creates_a_minimal_story_and_writes_work_units_json() {
    // Scenario: Dispatcher creates a minimal story and writes spec/work-units.json

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH' and title='User login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json contains a work unit AUTH-001 with type='story', status='backlog'
    let v = read_work_units(tmp.path());
    let wu = &v["workUnits"]["AUTH-001"];
    assert_eq!(wu["type"].as_str(), Some("story"));
    assert_eq!(wu["status"].as_str(), Some("backlog"));

    // @step And AUTH-001 has a non-empty createdAt and updatedAt
    assert!(!wu["createdAt"].as_str().unwrap_or("").is_empty());
    assert!(!wu["updatedAt"].as_str().unwrap_or("").is_empty());

    // @step And AUTH-001 has a children field equal to an empty array
    assert_eq!(wu["children"].as_array().map(Vec::len), Some(0));

    // @step And AUTH-001 does NOT contain a 'parent' key
    assert!(
        wu.get("parent").is_none(),
        "must not have parent key; got {wu}"
    );

    // @step And states.backlog contains 'AUTH-001'
    let backlog = v["states"]["backlog"].as_array().expect("backlog array");
    assert!(backlog.iter().any(|x| x.as_str() == Some("AUTH-001")));

    // @step And prefixCounters.AUTH equals 1
    assert_eq!(v["prefixCounters"]["AUTH"].as_u64(), Some(1));
}

#[test]
fn new_story_object_field_order_matches_ts_object_literal() {
    // Scenario: New story object field order matches the TS object literal

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH' and title='User login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then in the on-disk JSON the AUTH-001 keys appear in order id, title, type, status, createdAt, updatedAt, children
    let v = read_work_units(tmp.path());
    let keys: Vec<&str> = v["workUnits"]["AUTH-001"]
        .as_object()
        .expect("AUTH-001 object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "id",
            "title",
            "type",
            "status",
            "createdAt",
            "updatedAt",
            "children"
        ],
        "AUTH-001 key order must match TS object-literal insertion order"
    );
}

#[test]
fn dispatcher_stores_optional_description_after_updated_at() {
    // Scenario: Dispatcher stores an optional description after updatedAt

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH', title='User login', and description='Email + password'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login", "description": "Email + password"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json shows AUTH-001.description='Email + password'
    let v = read_work_units(tmp.path());
    assert_eq!(
        v["workUnits"]["AUTH-001"]["description"].as_str(),
        Some("Email + password")
    );

    // @step And in the on-disk JSON the 'updatedAt' key appears before the 'description' key
    let raw = read_work_units_raw(tmp.path());
    let upd = raw.find("\"updatedAt\"").expect("updatedAt key");
    let desc = raw.find("\"description\"").expect("description key");
    assert!(
        upd < desc,
        "updatedAt ({upd}) must appear before description ({desc})"
    );
}

#[test]
fn id_generation_increments_using_prefix_counters_high_water_mark() {
    // Scenario: ID generation increments using the prefixCounters high-water-mark

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH and an existing AUTH-001 story
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);
    write_work_units(
        tmp.path(),
        &build_work_units(&[("AUTH-001", "backlog", "")]),
    );

    // @step When I dispatch create-story with prefix='AUTH' and title='Second story'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "Second story"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json contains a work unit AUTH-002
    let v = read_work_units(tmp.path());
    assert!(
        v["workUnits"].get("AUTH-002").is_some(),
        "AUTH-002 must exist"
    );

    // @step And prefixCounters.AUTH equals 2
    assert_eq!(v["prefixCounters"]["AUTH"].as_u64(), Some(2));
}

#[test]
fn a_child_story_is_linked_to_its_parent_and_omits_children_array() {
    // Scenario: A child story is linked to its parent and omits the children array

    // @step Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and an existing parent AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);
    write_work_units(
        tmp.path(),
        &build_work_units(&[("AUTH-001", "backlog", "")]),
    );

    // @step When I dispatch create-story with prefix='AUTH', title='Child story', and parent='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "Child story", "parent": "AUTH-001"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json shows AUTH-002.parent='AUTH-001'
    let v = read_work_units(tmp.path());
    assert_eq!(
        v["workUnits"]["AUTH-002"]["parent"].as_str(),
        Some("AUTH-001")
    );

    // @step And AUTH-002 does NOT contain a 'children' key
    assert!(
        v["workUnits"]["AUTH-002"].get("children").is_none(),
        "child story must omit children key"
    );

    // @step And AUTH-001.children contains 'AUTH-002'
    let children = v["workUnits"]["AUTH-001"]["children"]
        .as_array()
        .expect("children array");
    assert!(children.iter().any(|x| x.as_str() == Some("AUTH-002")));
}

#[test]
fn an_epic_association_appends_the_story_id_to_the_epic_work_units_array() {
    // Scenario: An epic association appends the story id to the epic workUnits array

    // @step Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and spec/epics.json containing epic 'auth'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);
    write_epics(tmp.path(), &["auth"]);

    // @step When I dispatch create-story with prefix='AUTH', title='User login', and epic='auth'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login", "epic": "auth"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json shows AUTH-001.epic='auth'
    let v = read_work_units(tmp.path());
    assert_eq!(v["workUnits"]["AUTH-001"]["epic"].as_str(), Some("auth"));

    // @step And spec/epics.json shows epic 'auth' workUnits contains 'AUTH-001'
    let e = read_epics(tmp.path());
    let work_units = e["epics"]["auth"]["workUnits"]
        .as_array()
        .expect("epic workUnits array");
    assert!(work_units.iter().any(|x| x.as_str() == Some("AUTH-001")));
}

#[test]
fn dispatcher_rejects_a_missing_foundation_with_the_foundation_missing_message() {
    // Scenario: Dispatcher rejects a missing foundation with the foundation-missing message

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-story with prefix='AUTH' and title='User login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Project foundation not found'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Project foundation not found"),
        "expected foundation-missing message; got: {err}"
    );

    // @step And the error message contains the substring '<system-reminder>'
    assert!(
        err.contains("<system-reminder>"),
        "expected system-reminder in error; got: {err}"
    );

    // @step And spec/work-units.json does NOT exist
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "work-units.json must not be written when foundation is missing"
    );
}

#[test]
fn dispatcher_rejects_an_empty_title() {
    // Scenario: Dispatcher rejects an empty title

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH' and title=''
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "AUTH", "title": ""})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Title is required'
    let err = result.error.unwrap_or_default();
    assert!(err.contains("Title is required"), "got: {err}");
}

#[test]
fn dispatcher_rejects_an_unregistered_prefix() {
    // Scenario: Dispatcher rejects an unregistered prefix

    // @step Given a project root tempdir with spec/foundation.json present and an empty spec/prefixes.json
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &[]);

    // @step When I dispatch create-story with prefix='NOPE' and title='User login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "NOPE", "title": "User login"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Prefix 'NOPE' is not registered"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Prefix 'NOPE' is not registered"),
        "got: {err}"
    );
}

#[test]
fn dispatcher_rejects_a_non_existent_parent() {
    // Scenario: Dispatcher rejects a non-existent parent

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH', title='Child', and parent='AUTH-999'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "Child", "parent": "AUTH-999"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Parent story 'AUTH-999' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Parent story 'AUTH-999' does not exist"),
        "got: {err}"
    );
}

#[test]
fn dispatcher_rejects_exceeding_the_maximum_nesting_depth() {
    // Scenario: Dispatcher rejects exceeding the maximum nesting depth

    // @step Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and a three-level parent chain AUTH-001 -> AUTH-002 -> AUTH-003
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);
    write_work_units(
        tmp.path(),
        &build_work_units(&[
            ("AUTH-001", "backlog", ""),
            ("AUTH-002", "backlog", "AUTH-001"),
            ("AUTH-003", "backlog", "AUTH-002"),
        ]),
    );

    // @step When I dispatch create-story with prefix='AUTH', title='Too deep', and parent='AUTH-003'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "Too deep", "parent": "AUTH-003"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Maximum nesting depth (3) exceeded'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Maximum nesting depth (3) exceeded"),
        "got: {err}"
    );
}

#[test]
fn dispatcher_rejects_a_non_existent_epic() {
    // Scenario: Dispatcher rejects a non-existent epic

    // @step Given a project root tempdir with spec/foundation.json present, spec/prefixes.json registering prefix AUTH, and an empty spec/epics.json
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);
    write_epics(tmp.path(), &[]);

    // @step When I dispatch create-story with prefix='AUTH', title='User login', and epic='ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login", "epic": "ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Epic 'ghost' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(err.contains("Epic 'ghost' does not exist"), "got: {err}");
}

#[test]
fn dispatcher_response_text_renders_the_success_block_and_example_mapping_reminder() {
    // Scenario: Dispatcher response text renders the success block and Example-Mapping reminder

    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefixes(tmp.path(), &["AUTH"]);

    // @step When I dispatch create-story with prefix='AUTH' and title='User login'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "AUTH", "title": "User login"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Created story AUTH-001'
    assert!(
        result.data.lines().any(|l| l == "✓ Created story AUTH-001"),
        "missing checkmark line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the line '  Title: User login'
    assert!(
        result.data.lines().any(|l| l == "  Title: User login"),
        "missing title line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the substring '<system-reminder>'
    assert!(
        result.data.contains("<system-reminder>"),
        "missing system-reminder; got:\n{}",
        result.data
    );
}
