#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-diagram-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-diagram`
// (RPC-216). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "delete-diagram".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn read_foundation(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/foundation.json"))
        .expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn foundation_with_diagrams(titles: &[&str]) -> Value {
    let diagrams: Vec<Value> = titles
        .iter()
        .map(|t| json!({"title": *t, "mermaidCode": format!("graph TD\n  {t}-->X")}))
        .collect();
    json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": diagrams
    })
}

// ---------- scenarios ----------

#[test]
fn dispatcher_removes_an_existing_diagram_by_title() {
    // Scenario: Dispatcher removes an existing diagram by title

    // @step Given spec/foundation.json contains a diagram titled 'Component Flow' with mermaidCode='graph TD\n  A-->B'
    let tmp = TempDir::new().expect("tempdir");
    let f = json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": [
            {"title": "Component Flow", "mermaidCode": "graph TD\n  A-->B"}
        ]
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch delete-diagram with section='Architecture' title='Component Flow'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Component Flow"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");

    // @step And spec/foundation.json architectureDiagrams is empty
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 0, "expected empty, got {diagrams:?}");

    // @step And the response message contains the substring "Deleted diagram 'Component Flow' from section 'Architecture'"
    assert!(
        result.data.contains("Deleted diagram 'Component Flow' from section 'Architecture'"),
        "missing canonical success text; got:\n{}",
        result.data
    );
}

#[test]
fn dispatcher_fails_when_foundation_json_is_missing() {
    // Scenario: Dispatcher fails when spec/foundation.json is missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch delete-diagram with section='Architecture' title='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'foundation.json not found: spec/foundation.json'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("foundation.json not found: spec/foundation.json"),
        "missing canonical error text; got: {msg}"
    );

    // @step And the file spec/foundation.json still does NOT exist
    assert!(
        !tmp.path().join("spec/foundation.json").exists(),
        "delete-diagram must NOT auto-create foundation.json"
    );
}

#[test]
fn dispatcher_fails_when_title_is_not_found() {
    // Scenario: Dispatcher fails when title is not found

    // @step Given spec/foundation.json contains a diagram titled 'Existing' with mermaidCode='graph TD\n  A-->B'
    let tmp = TempDir::new().expect("tempdir");
    let f = json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": [
            {"title": "Existing", "mermaidCode": "graph TD\n  A-->B"}
        ]
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch delete-diagram with section='Architecture' title='Missing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Missing"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Diagram 'Missing' not found in section 'Architecture'"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Diagram 'Missing' not found in section 'Architecture'"),
        "missing canonical error text; got: {msg}"
    );

    // @step And spec/foundation.json architectureDiagrams still contains the diagram titled 'Existing'
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0]["title"].as_str(), Some("Existing"));
}

#[test]
fn dispatcher_removes_only_middle_entry_from_three_diagrams() {
    // Scenario: Dispatcher removes only the middle entry from a list of three diagrams

    // @step Given spec/foundation.json contains diagrams titled 'First', 'Middle', 'Last' in that order
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_diagrams(&["First", "Middle", "Last"]));

    // @step When I dispatch delete-diagram with section='Architecture' title='Middle'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "Middle"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json architectureDiagrams has length 2
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 2);

    // @step And the remaining diagrams are ['First', 'Last'] in that order
    assert_eq!(diagrams[0]["title"].as_str(), Some("First"));
    assert_eq!(diagrams[1]["title"].as_str(), Some("Last"));
}

#[test]
fn dispatcher_leaves_architecture_diagrams_as_empty_after_removing_only_entry() {
    // Scenario: Dispatcher leaves architectureDiagrams as an empty array after removing the only entry

    // @step Given spec/foundation.json contains exactly one diagram titled 'OnlyOne'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_diagrams(&["OnlyOne"]));

    // @step When I dispatch delete-diagram with section='Architecture' title='OnlyOne'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "OnlyOne"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json architectureDiagrams equals []
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 0);

    // @step And spec/foundation.json still has its 'project' object intact
    assert_eq!(data["project"]["name"].as_str(), Some("fspec"));
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Dispatcher preserves unknown top-level fields on write

    // @step Given spec/foundation.json contains a diagram titled 'X' and a top-level 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let f = json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": [
            {"title": "X", "mermaidCode": "graph TD\n  A-->B"}
        ],
        "experiments": {"alpha": true, "beta": [1, 2, 3]}
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch delete-diagram with section='Architecture' title='X'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "Architecture", "title": "X"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json still contains the 'experiments' key with its original value
    let data = read_foundation(tmp.path());
    assert_eq!(data["experiments"]["alpha"].as_bool(), Some(true));
    assert_eq!(data["experiments"]["beta"][0].as_u64(), Some(1));
}

#[test]
fn dispatcher_matches_by_title_only_section_arg_informational() {
    // Scenario: Dispatcher matches by title only — section argument is informational (Framing A)

    // @step Given spec/foundation.json contains a diagram titled 'OneTitle' (no 'section' field on the entry)
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_diagrams(&["OneTitle"]));

    // @step When I dispatch delete-diagram with section='AnythingElse' title='OneTitle'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "AnythingElse", "title": "OneTitle"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "Framing A: matching is by title only; got {result:?}"
    );

    // @step And spec/foundation.json architectureDiagrams is empty
    let data = read_foundation(tmp.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 0);

    // @step And the response message echoes the supplied section 'AnythingElse'
    assert!(
        result.data.contains("AnythingElse"),
        "response must echo supplied section; got:\n{}",
        result.data
    );
}
