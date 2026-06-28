#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-foundation-schema-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `validate-foundation-schema` (RPC-321). Each scenario maps to exactly one
// #[test] fn with @step comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a 1-arg NotYetPorted stub, so
// every dispatch returns FspecCoreError::NotYetPorted. These tests are RED
// until PHASE C lands the real impl + the supervisor re-points the dispatcher.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate-foundation-schema".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// A schema-valid minimal foundation (version, project, problemSpace,
/// solutionSpace with one capability) — mirrors the native validator's
/// own `valid_foundation()` fixture.
fn valid_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
        "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
    })
}

/// Parse the dispatcher `data` field as JSON (the structured `{success,...}`
/// envelope the command emits, mirroring the show_feature precedent).
fn data_json(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}; got:\n{}", result.data))
}

// ───────── scenarios ─────────

#[test]
fn validates_a_schema_valid_foundation_and_reports_success() {
    // Scenario: Validates a schema-valid foundation and reports success

    // @step Given spec/foundation.json contains a schema-valid minimal foundation with version, project, problemSpace, and solutionSpace with one capability
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/foundation.json",
        &serde_json::to_string_pretty(&valid_foundation()).unwrap(),
    );

    // @step When I dispatch the validate-foundation-schema command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true with the output '✓ foundation.json is valid according to the schema'
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(
        data["output"].as_str(),
        Some("✓ foundation.json is valid according to the schema"),
        "got data: {data}"
    );
}

#[test]
fn reports_a_friendly_error_when_foundation_json_is_missing() {
    // Scenario: Reports a friendly error when foundation.json is missing

    // @step Given an empty project root directory with no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/foundation.json").exists());

    // @step When I dispatch the validate-foundation-schema command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with the error 'foundation.json not found in spec/ directory'
    // (Per the show_feature precedent: recoverable failures live inside the
    // structured `data` envelope; the outer dispatch envelope stays success=true.)
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");
    assert_eq!(
        data["error"].as_str(),
        Some("foundation.json not found in spec/ directory"),
        "got data: {data}"
    );
}

#[test]
fn renders_the_min_items_special_case_error_for_an_empty_capabilities_array() {
    // Scenario: Renders the minItems special-case error for an empty capabilities array

    // @step Given spec/foundation.json is valid except solutionSpace.capabilities is an empty array
    let tmp = TempDir::new().expect("tempdir");
    let mut f = valid_foundation();
    f["solutionSpace"]["capabilities"] = json!([]);
    write_file(
        tmp.path(),
        "spec/foundation.json",
        &serde_json::to_string_pretty(&f).unwrap(),
    );

    // @step When I dispatch the validate-foundation-schema command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with the error 'Field solutionSpace.capabilities must have at least 1 items (found 0)'
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");
    assert_eq!(
        data["error"].as_str(),
        Some("Field solutionSpace.capabilities must have at least 1 items (found 0)"),
        "got data: {data}"
    );
}

#[test]
fn renders_a_required_property_error_when_a_top_level_field_is_missing() {
    // Scenario: Renders a required-property error when a top-level field is missing

    // @step Given spec/foundation.json is valid except it is missing the required solutionSpace property
    let tmp = TempDir::new().expect("tempdir");
    let mut f = valid_foundation();
    f.as_object_mut().unwrap().remove("solutionSpace");
    write_file(
        tmp.path(),
        "spec/foundation.json",
        &serde_json::to_string_pretty(&f).unwrap(),
    );

    // @step When I dispatch the validate-foundation-schema command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with the error "#/required: must have required property 'solutionSpace'"
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");
    assert_eq!(
        data["error"].as_str(),
        // TS Ajv renders root-level errors via `instancePath || schemaPath`;
        // instancePath is "" at root so it falls back to schemaPath '#/required'.
        // Captured byte-exact from `node dist/index.js validate-foundation-schema`.
        Some("#/required: must have required property 'solutionSpace'"),
        "got data: {data}"
    );
}

#[test]
fn reports_a_friendly_error_when_foundation_json_contains_malformed_json() {
    // Scenario: Reports a friendly error when foundation.json contains malformed JSON

    // @step Given spec/foundation.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/foundation.json", "{ not json");

    // @step When I dispatch the validate-foundation-schema command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error beginning 'Failed to validate foundation schema:'
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");
    let err = data["error"].as_str().expect("error string");
    assert!(
        err.starts_with("Failed to validate foundation schema:"),
        "expected malformed-json prefix; got: {err}"
    );
}
