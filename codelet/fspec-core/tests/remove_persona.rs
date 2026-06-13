#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-persona-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-persona`
// (RPC-277). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-persona".to_string(),
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

fn persona(name: &str) -> Value {
    json!({"name": name, "description": "d", "goals": []})
}

fn foundation_with(personas: Value) -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "personas": personas
    })
}

// ---------- scenarios ----------

#[test]
fn remove_existing_persona_by_exact_name() {
    // Scenario: Remove an existing persona by exact name

    // @step Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "foundation.json",
        &foundation_with(json!([persona("Primary User"), persona("Admin")])),
    );

    // @step When I dispatch remove-persona with name='Admin'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Admin"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains fileName='foundation.json'
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["fileName"].as_str(), Some("foundation.json"));

    // @step And the returned data contains name='Admin'
    assert_eq!(data["name"].as_str(), Some("Admin"));

    // @step And spec/foundation.json on disk shows personas has length 1
    let f = read_json(tmp.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("array");
    assert_eq!(personas.len(), 1);

    // @step And spec/foundation.json on disk shows the only persona has name='Primary User'
    assert_eq!(personas[0]["name"].as_str(), Some("Primary User"));
}

#[test]
fn draft_precedence_routes_removal_to_draft() {
    // Scenario: Draft precedence routes the removal to foundation.json.draft

    // @step Given a project root tempdir with spec/foundation.json.draft containing persona 'Drafted'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "foundation.json.draft",
        &foundation_with(json!([persona("Drafted")])),
    );

    // @step When I dispatch remove-persona with name='Drafted'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Drafted"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the returned data contains fileName='foundation.json.draft'
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["fileName"].as_str(), Some("foundation.json.draft"));

    // @step And spec/foundation.json.draft on disk shows personas has length 0
    let d = read_json(tmp.path(), "foundation.json.draft");
    assert_eq!(d["personas"].as_array().expect("array").len(), 0);
}

#[test]
fn removing_nonexistent_name_lists_available_personas() {
    // Scenario: Removing a non-existent name lists the available personas

    // @step Given a project root tempdir with spec/foundation.json containing personas 'Primary User' and 'Admin'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "foundation.json",
        &foundation_with(json!([persona("Primary User"), persona("Admin")])),
    );
    let before = read_raw(tmp.path(), "foundation.json");

    // @step When I dispatch remove-persona with name='Ghost'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Ghost"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Persona "Ghost" not found'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Persona \"Ghost\" not found"),
        "missing not-found text; got: {msg}"
    );

    // @step And the error message contains the substring 'Available personas: Primary User, Admin'
    assert!(
        msg.contains("Available personas: Primary User, Admin"),
        "missing available list; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_raw(tmp.path(), "foundation.json"), before);
}

#[test]
fn removing_from_empty_personas_reports_none_exist() {
    // Scenario: Removing from an empty personas array reports that no personas exist

    // @step Given a project root tempdir with spec/foundation.json whose personas array is empty
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "foundation.json", &foundation_with(json!([])));
    let before = read_raw(tmp.path(), "foundation.json");

    // @step When I dispatch remove-persona with name='Admin'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Admin"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Persona "Admin" not found'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Persona \"Admin\" not found"),
        "missing not-found text; got: {msg}"
    );

    // @step And the error message contains the substring 'No personas exist in foundation'
    assert!(
        msg.contains("No personas exist in foundation"),
        "missing none-exist text; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_raw(tmp.path(), "foundation.json"), before);
}

#[test]
fn missing_foundation_and_draft_surface_not_found_and_create_nothing() {
    // Scenario: Missing foundation file and draft surface the not-found error and create nothing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch remove-persona with name='Admin'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Admin"})));

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
fn name_matching_is_case_sensitive() {
    // Scenario: Name matching is case-sensitive

    // @step Given a project root tempdir with spec/foundation.json containing persona 'Primary User'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "foundation.json",
        &foundation_with(json!([persona("Primary User")])),
    );
    let before = read_raw(tmp.path(), "foundation.json");

    // @step When I dispatch remove-persona with name='primary user'
    let result = dispatch_command(req(tmp.path(), json!({"name": "primary user"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Persona "primary user" not found'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Persona \"primary user\" not found"),
        "missing not-found text; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_raw(tmp.path(), "foundation.json"), before);
}

#[test]
fn only_first_matching_persona_removed_when_duplicated() {
    // Scenario: Only the first matching persona is removed when names are duplicated

    // @step Given a project root tempdir with spec/foundation.json containing two personas both named 'Dup'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "foundation.json",
        &foundation_with(json!([persona("Dup"), persona("Dup")])),
    );

    // @step When I dispatch remove-persona with name='Dup'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Dup"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows personas has length 1
    let f = read_json(tmp.path(), "foundation.json");
    let personas = f["personas"].as_array().expect("array");
    assert_eq!(personas.len(), 1);

    // @step And spec/foundation.json on disk shows the only persona has name='Dup'
    assert_eq!(personas[0]["name"].as_str(), Some("Dup"));
}

#[test]
fn written_file_uses_2_space_indent_trailing_newline_and_preserves_unknown_fields() {
    // Scenario: The written file uses 2-space indentation, a trailing newline, and preserves unknown top-level fields

    // @step Given a project root tempdir with spec/foundation.json containing a custom top-level field "customKey" and personas 'Primary User' and 'Admin'
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_with(json!([persona("Primary User"), persona("Admin")]));
    f["customKey"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_file(tmp.path(), "foundation.json", &f);

    // @step When I dispatch remove-persona with name='Admin'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Admin"})));

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
