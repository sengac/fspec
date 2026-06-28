#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/suggest-dependencies-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of the
// `suggest-dependencies` command (RPC-309). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the `suggest-dependencies` command is still a stub returning
// FspecCoreError::NotYetPorted, so every assertion below FAILS until Phase C
// wires the real implementation.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "suggest-dependencies".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_raw_work_units(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a `spec/work-units.json` payload with the given (id, title, extra)
/// entries. `extra` carries relationship arrays (dependsOn / blockedBy) and/or
/// epic as raw JSON fragments merged onto the canonical work-unit shape.
fn work_units_with(entries: &[(&str, &str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, title, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String((*title).to_string()));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String("backlog".to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Value::Object(map) = extra {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
        wus.insert((*id).to_string(), Value::Object(obj));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

/// Parse the dispatcher JSON `data` body and return the `suggestions` array.
fn suggestions_of(data: &str) -> Vec<Value> {
    let parsed: Value = serde_json::from_str(data).expect("data must be JSON");
    parsed
        .get("suggestions")
        .and_then(Value::as_array)
        .cloned()
        .expect("root object must have suggestions array")
}

// ───────── scenarios ─────────

#[test]
fn returns_empty_suggestions_array_when_work_units_json_is_auto_created_in_empty_workspace() {
    // Scenario: Returns empty suggestions array when work-units.json is auto-created in an empty workspace

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON has suggestions=[]
    assert!(
        suggestions_of(&result.data).is_empty(),
        "expected empty suggestions, got:\n{}",
        result.data
    );

    // @step And spec/work-units.json exists after the call
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created"
    );
}

#[test]
fn sequential_ids_in_the_same_prefix_produce_a_medium_confidence_dependson_suggestion() {
    // Scenario: Sequential IDs in the same prefix produce a medium-confidence dependsOn suggestion

    // @step Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has a suggestion with from='AUTH-002' to='AUTH-001' type='dependsOn' confidence='medium'
    let suggestions = suggestions_of(&result.data);
    let found = suggestions.iter().find(|s| {
        s["from"] == json!("AUTH-002")
            && s["to"] == json!("AUTH-001")
            && s["type"] == json!("dependsOn")
            && s["confidence"] == json!("medium")
    });
    assert!(
        found.is_some(),
        "expected AUTH-002->AUTH-001 dependsOn medium, got:\n{}",
        result.data
    );

    // @step And that suggestion reason contains 'sequential IDs in AUTH prefix'
    let reason = found.unwrap()["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("sequential IDs in AUTH prefix"),
        "reason missing substring; got: {reason}"
    );
}

#[test]
fn a_test_work_unit_depends_on_a_matching_build_work_unit_with_high_confidence() {
    // Scenario: A Test work unit depends on a matching Build work unit with high confidence

    // @step Given spec/work-units.json contains BUILD-001 titled 'Build authentication' and TEST-001 titled 'Test authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("BUILD-001", "Build authentication", json!({})),
            ("TEST-001", "Test authentication", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has a suggestion with from='TEST-001' to='BUILD-001' type='dependsOn' confidence='high'
    let suggestions = suggestions_of(&result.data);
    let found = suggestions.iter().find(|s| {
        s["from"] == json!("TEST-001")
            && s["to"] == json!("BUILD-001")
            && s["type"] == json!("dependsOn")
            && s["confidence"] == json!("high")
    });
    assert!(
        found.is_some(),
        "expected TEST-001->BUILD-001 dependsOn high, got:\n{}",
        result.data
    );

    // @step And that suggestion reason contains 'test work depends on build work'
    let reason = found.unwrap()["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("test work depends on build work"),
        "reason missing substring; got: {reason}"
    );
}

#[test]
fn a_feature_work_unit_depends_on_a_same_prefix_infrastructure_work_unit_with_high_confidence() {
    // Scenario: A feature work unit depends on a same-prefix infrastructure work unit with high confidence

    // @step Given spec/work-units.json contains FEAT-001 titled 'Database schema setup' and FEAT-002 titled 'Add user features'
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("FEAT-001", "Database schema setup", json!({})),
            ("FEAT-002", "Add user features", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has a suggestion with from='FEAT-002' to='FEAT-001' type='dependsOn' confidence='high'
    let suggestions = suggestions_of(&result.data);
    let found = suggestions.iter().find(|s| {
        s["from"] == json!("FEAT-002")
            && s["to"] == json!("FEAT-001")
            && s["type"] == json!("dependsOn")
            && s["confidence"] == json!("high")
    });
    assert!(
        found.is_some(),
        "expected FEAT-002->FEAT-001 dependsOn high, got:\n{}",
        result.data
    );

    // @step And that suggestion reason contains 'infrastructure work (schema/migration) should complete before feature work'
    let reason = found.unwrap()["reason"].as_str().unwrap_or("");
    assert!(
        reason
            .contains("infrastructure work (schema/migration) should complete before feature work"),
        "reason missing substring; got: {reason}"
    );
}

#[test]
fn specific_patterns_override_the_generic_sequential_suggestion_for_the_same_pair() {
    // Scenario: Specific patterns override the generic sequential suggestion for the same pair

    // @step Given spec/work-units.json contains BUILD-001 titled 'Build authentication' and BUILD-002 titled 'Test authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("BUILD-001", "Build authentication", json!({})),
            ("BUILD-002", "Test authentication", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has exactly one suggestion with from='BUILD-002' to='BUILD-001'
    let suggestions = suggestions_of(&result.data);
    let matching: Vec<&Value> = suggestions
        .iter()
        .filter(|s| s["from"] == json!("BUILD-002") && s["to"] == json!("BUILD-001"))
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "expected exactly one BUILD-002->BUILD-001 suggestion, got:\n{}",
        result.data
    );

    // @step And that suggestion confidence='high'
    assert_eq!(
        matching[0]["confidence"],
        json!("high"),
        "expected high confidence, got:\n{}",
        result.data
    );
}

#[test]
fn existing_dependson_relationship_excludes_the_sequential_suggestion() {
    // Scenario: Existing dependsOn relationship excludes the sequential suggestion

    // @step Given spec/work-units.json contains AUTH-001 and AUTH-002 where AUTH-002 already lists AUTH-001 in dependsOn
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({ "dependsOn": ["AUTH-001"] })),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has suggestions=[]
    assert!(
        suggestions_of(&result.data).is_empty(),
        "expected empty suggestions, got:\n{}",
        result.data
    );
}

#[test]
fn the_unimplemented_same_epic_relatesto_rule_produces_no_suggestions() {
    // Scenario: The unimplemented same-epic relatesTo rule produces no suggestions

    // @step Given spec/work-units.json contains XX-001 and YY-001 in epic 'auth' with different prefixes and no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("XX-001", "Work xx", json!({ "epic": "auth" })),
            ("YY-001", "Work yy", json!({ "epic": "auth" })),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the returned JSON has suggestions=[]
    assert!(
        suggestions_of(&result.data).is_empty(),
        "expected empty suggestions, got:\n{}",
        result.data
    );
}

#[test]
fn json_suggestion_field_declaration_order_is_from_to_type_reason_confidence() {
    // Scenario: JSON suggestion field declaration order is from, to, type, reason, confidence

    // @step Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the first suggestion object's field declaration order is from, to, type, reason, confidence
    let data = &result.data;
    let p_from = data.find("\"from\"").expect("from key present");
    let p_to = data.find("\"to\"").expect("to key present");
    let p_type = data.find("\"type\"").expect("type key present");
    let p_reason = data.find("\"reason\"").expect("reason key present");
    let p_conf = data.find("\"confidence\"").expect("confidence key present");
    assert!(
        p_from < p_to && p_to < p_type && p_type < p_reason && p_reason < p_conf,
        "field order must be from,to,type,reason,confidence; got:\n{data}"
    );
}

#[test]
fn default_text_output_renders_a_numbered_summary() {
    // Scenario: Default text output renders a numbered summary

    // @step Given spec/work-units.json contains AUTH-001 and AUTH-002 with no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "Work one", json!({})),
            ("AUTH-002", "Work two", json!({})),
        ]),
    );

    // @step When I dispatch suggest-dependencies with default text output
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the rendered text contains 'Found 1 dependency suggestion(s):'
    assert!(
        result.data.contains("Found 1 dependency suggestion(s):"),
        "got:\n{}",
        result.data
    );

    // @step And the rendered text contains 'AUTH-002'
    assert!(result.data.contains("AUTH-002"), "got:\n{}", result.data);

    // @step And the rendered text contains 'Confidence: MEDIUM'
    assert!(
        result.data.contains("Confidence: MEDIUM"),
        "got:\n{}",
        result.data
    );
}

#[test]
fn default_text_output_renders_the_empty_sentinel_when_there_are_no_suggestions() {
    // Scenario: Default text output renders the empty sentinel when there are no suggestions

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch suggest-dependencies with default text output
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the rendered text contains 'No dependency suggestions found.'
    assert!(
        result.data.contains("No dependency suggestions found."),
        "got:\n{}",
        result.data
    );
}

#[test]
fn escalates_malformed_work_units_json_as_a_structured_parse_error() {
    // Scenario: Escalates malformed work-units.json as a structured parse error

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(tmp.path(), "{ not json");

    // @step When I dispatch suggest-dependencies against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing substring: {msg}"
    );
}
