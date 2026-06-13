#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-persona-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-persona`
// (RPC-186). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-persona".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(project_root: &Path, name: &str, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join(name),
        serde_json::to_string_pretty(value).expect("ser") + "\n",
    )
    .expect("write file");
}

fn read_json(project_root: &Path, name: &str) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join(name)).expect("read file");
    serde_json::from_str(&raw).expect("parse json")
}

fn read_raw(project_root: &Path, name: &str) -> String {
    fs::read_to_string(project_root.join("spec").join(name)).expect("read raw")
}

/// foundation with a single real persona "Primary User".
fn foundation_one_real() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "personas": [
            {"name": "Primary User", "description": "User description", "goals": ["User goal"]}
        ]
    })
}

// ---------- scenarios ----------

#[test]
fn append_persona_to_existing_foundation_with_real_personas() {
    // Scenario: Append a persona to an existing foundation.json with real personas

    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "foundation.json", &foundation_one_real());

    // @step When I dispatch add-persona with name='QA Engineer', description='Tests features', goals=['Catch regressions']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "QA Engineer", "description": "Tests features", "goals": ["Catch regressions"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains fileName='foundation.json'
    let data: Value = serde_json::from_str(&result.data).expect("parse result data");
    assert_eq!(data["fileName"].as_str(), Some("foundation.json"));

    // @step And the returned data contains removedPlaceholders=0
    assert_eq!(data["removedPlaceholders"].as_u64(), Some(0));

    // @step And spec/foundation.json on disk shows personas has length 2
    let f = read_json(tmp.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("personas array");
    assert_eq!(personas.len(), 2, "expected 2 personas, got {personas:?}");

    // @step And spec/foundation.json on disk shows the last persona has name='QA Engineer', description='Tests features', goals=['Catch regressions']
    let last = &personas[1];
    assert_eq!(last["name"].as_str(), Some("QA Engineer"));
    assert_eq!(last["description"].as_str(), Some("Tests features"));
    assert_eq!(last["goals"], json!(["Catch regressions"]));
}

#[test]
fn multiple_goals_persisted_in_supplied_order() {
    // Scenario: Multiple repeated goals are persisted in supplied order

    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "foundation.json", &foundation_one_real());

    // @step When I dispatch add-persona with name='Founder', description='Runs the company', goals=['Ship fast', 'Stay safe']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Founder", "description": "Runs the company", "goals": ["Ship fast", "Stay safe"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows the last persona has goals=['Ship fast', 'Stay safe']
    let f = read_json(tmp.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("array");
    let last = personas.last().expect("at least one");
    assert_eq!(last["goals"], json!(["Ship fast", "Stay safe"]));

    // @step And the returned data contains name='Founder', description='Runs the company', goals=['Ship fast', 'Stay safe']
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["name"].as_str(), Some("Founder"));
    assert_eq!(data["description"].as_str(), Some("Runs the company"));
    assert_eq!(data["goals"], json!(["Ship fast", "Stay safe"]));
}

#[test]
fn persona_with_no_goals_stored_with_empty_array() {
    // Scenario: A persona with no goals is stored with an empty goals array

    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User"
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "foundation.json", &foundation_one_real());

    // @step When I dispatch add-persona with name='Observer', description='Just watches' and no goals
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Observer", "description": "Just watches"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows the last persona has goals=[]
    let f = read_json(tmp.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("array");
    let last = personas.last().expect("at least one");
    assert_eq!(last["goals"], json!([]), "goals must be empty array");
}

#[test]
fn draft_precedence_routes_write_to_draft() {
    // Scenario: Draft precedence routes the write to foundation.json.draft

    // @step Given a project root tempdir with both spec/foundation.json and spec/foundation.json.draft present
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "foundation.json", &foundation_one_real());
    let mut draft = foundation_one_real();
    draft["personas"] = json!([]);
    write_file(tmp.path(), "foundation.json.draft", &draft);
    let final_before = read_raw(tmp.path(), "foundation.json");

    // @step When I dispatch add-persona with name='Drafted', description='Lives in the draft', goals=['Goal A']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Drafted", "description": "Lives in the draft", "goals": ["Goal A"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the returned data contains fileName='foundation.json.draft'
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["fileName"].as_str(), Some("foundation.json.draft"));

    // @step And spec/foundation.json.draft on disk shows personas includes a persona named 'Drafted'
    let d = read_json(tmp.path(), "foundation.json.draft");
    let names: Vec<&str> = d["personas"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Drafted"),
        "draft must contain Drafted; got {names:?}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_raw(tmp.path(), "foundation.json"), final_before);
}

#[test]
fn all_placeholder_personas_cleared_before_real_added() {
    // Scenario: An all-placeholder personas array is cleared before the real persona is added

    // @step Given a project root tempdir with spec/foundation.json whose only persona is named '[QUESTION: Who uses this?]'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_one_real();
    f["personas"] = json!([
        {"name": "[QUESTION: Who uses this?]", "description": "d", "goals": []}
    ]);
    write_file(tmp.path(), "foundation.json", &f);

    // @step When I dispatch add-persona with name='Developer', description='Builds features', goals=['Ship quality code']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Developer", "description": "Builds features", "goals": ["Ship quality code"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the returned data contains removedPlaceholders=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["removedPlaceholders"].as_u64(), Some(1));

    // @step And spec/foundation.json on disk shows personas has length 1
    let out = read_json(tmp.path(), "foundation.json");
    let personas = out["personas"].as_array().expect("array");
    assert_eq!(
        personas.len(),
        1,
        "placeholder must be cleared; got {personas:?}"
    );

    // @step And spec/foundation.json on disk shows the only persona has name='Developer'
    assert_eq!(personas[0]["name"].as_str(), Some("Developer"));
}

#[test]
fn real_persona_alongside_placeholder_suppresses_removal() {
    // Scenario: A real persona alongside a placeholder suppresses placeholder removal

    // @step Given a project root tempdir with spec/foundation.json containing one real persona "Primary User" and one placeholder persona '[DETECTED: Admin]'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_one_real();
    f["personas"] = json!([
        {"name": "Primary User", "description": "User description", "goals": ["User goal"]},
        {"name": "[DETECTED: Admin]", "description": "d", "goals": []}
    ]);
    write_file(tmp.path(), "foundation.json", &f);

    // @step When I dispatch add-persona with name='Developer', description='Builds features', goals=['Ship quality code']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Developer", "description": "Builds features", "goals": ["Ship quality code"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the returned data contains removedPlaceholders=0
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["removedPlaceholders"].as_u64(), Some(0));

    // @step And spec/foundation.json on disk shows personas has length 3
    let out = read_json(tmp.path(), "foundation.json");
    let personas = out["personas"].as_array().expect("array");
    assert_eq!(personas.len(), 3, "no removal expected; got {personas:?}");
}

#[test]
fn missing_foundation_and_draft_surface_not_found_and_create_nothing() {
    // Scenario: Missing foundation file and draft surface the not-found error and create nothing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-persona with name='Nobody', description='No file', goals=[]
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Nobody", "description": "No file", "goals": []}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "foundation.json not found"
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("foundation.json not found"),
        "missing canonical error; got: {msg}"
    );

    // @step And spec/foundation.json does not exist on disk
    assert!(!tmp.path().join("spec/foundation.json").exists());

    // @step And spec/foundation.json.draft does not exist on disk
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());
}

#[test]
fn foundation_with_no_personas_key_initializes_then_appends() {
    // Scenario: A foundation.json with no personas key initializes the array then appends

    // @step Given a project root tempdir with spec/foundation.json that has no personas key
    let tmp = TempDir::new().expect("tempdir");
    let f = json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"}
    });
    write_file(tmp.path(), "foundation.json", &f);

    // @step When I dispatch add-persona with name='First', description='Initial persona', goals=['Goal X']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "First", "description": "Initial persona", "goals": ["Goal X"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows personas has length 1
    let out = read_json(tmp.path(), "foundation.json");
    let personas = out["personas"].as_array().expect("array");
    assert_eq!(personas.len(), 1);

    // @step And spec/foundation.json on disk shows the only persona has name='First'
    assert_eq!(personas[0]["name"].as_str(), Some("First"));
}

#[test]
fn written_file_uses_2_space_indent_trailing_newline_and_preserves_unknown_fields() {
    // Scenario: The written file uses 2-space indentation, a trailing newline, and preserves unknown top-level fields

    // @step Given a project root tempdir with spec/foundation.json containing a custom top-level field "customKey" and one real persona "Primary User"
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_one_real();
    f["customKey"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_file(tmp.path(), "foundation.json", &f);

    // @step When I dispatch add-persona with name='QA Engineer', description='Tests features', goals=['Catch regressions']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "QA Engineer", "description": "Tests features", "goals": ["Catch regressions"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk ends with a single trailing newline
    let raw = read_raw(tmp.path(), "foundation.json");
    assert!(raw.ends_with('\n'), "must end with newline");
    assert!(!raw.ends_with("\n\n"), "must end with a SINGLE newline");

    // @step And spec/foundation.json on disk is indented with 2 spaces
    assert!(
        raw.contains("\n  \"version\""),
        "expected 2-space indentation; got:\n{raw}"
    );

    // @step And spec/foundation.json on disk still contains the top-level field "customKey" unchanged
    let out = read_json(tmp.path(), "foundation.json");
    assert_eq!(out["customKey"]["alpha"].as_bool(), Some(true));
    assert_eq!(out["customKey"]["beta"][0].as_u64(), Some(1));
}
