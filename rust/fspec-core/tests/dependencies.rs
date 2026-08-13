#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/dependencies-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of the `dependencies`
// command (RPC-224). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the `dependencies` command is still a stub returning
// FspecCoreError::NotYetPorted, so every success-path assertion below FAILS
// until Phase C wires the real implementation. The missing-work-unit scenario
// expects success=false but with a 'does not exist' substring it will NOT yet
// satisfy (the stub error says 'not yet ported'), so it also fails red.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "dependencies".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a `spec/work-units.json` payload with the given (id, extra) entries.
/// `extra` carries the relationship arrays (and/or legacy top-level arrays)
/// as raw JSON fragments merged onto the canonical work-unit shape.
fn work_units_with(entries: &[(&str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
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

// ───────── scenarios ─────────

#[test]
fn dispatch_unit_with_all_four_relationship_types_renders_header_and_four_lines_in_fixed_order() {
    // Scenario: Dispatch dependencies for a unit with all four relationship types renders the header and all four lines in fixed order

    // @step Given spec/work-units.json contains AUTH-001 whose relationships are blocks=['AUTH-002','AUTH-003'], blockedBy=['INFRA-001'], dependsOn=['SCHEMA-001'], relatesTo=['DOC-001']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            (
                "AUTH-001",
                json!({
                    "relationships": {
                        "blocks": ["AUTH-002", "AUTH-003"],
                        "blockedBy": ["INFRA-001"],
                        "dependsOn": ["SCHEMA-001"],
                        "relatesTo": ["DOC-001"]
                    }
                }),
            ),
            ("AUTH-002", json!({})),
            ("AUTH-003", json!({})),
            ("INFRA-001", json!({})),
            ("SCHEMA-001", json!({})),
            ("DOC-001", json!({})),
        ]),
    );

    // @step When I dispatch dependencies with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the data field equals "Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001\n"
    let expected = "Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001\n";
    assert_eq!(result.data, expected, "got:\n{}", result.data);
}

#[test]
fn dispatch_unit_with_no_relationships_renders_only_the_header_line() {
    // Scenario: Dispatch dependencies for a unit with no relationships renders only the header line

    // @step Given spec/work-units.json contains MCP-001 with no relationship fields
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("MCP-001", json!({}))]));

    // @step When I dispatch dependencies with workUnitId='MCP-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "MCP-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the data field equals "Dependencies for MCP-001:\n"
    assert_eq!(
        result.data, "Dependencies for MCP-001:\n",
        "got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_with_graph_renders_a_depth_first_blocks_tree_with_increasing_indentation() {
    // Scenario: Dispatch dependencies with --graph renders a depth-first blocks tree with increasing indentation

    // @step Given spec/work-units.json contains AUTH-001 with blocks=['AUTH-002','AUTH-003'], AUTH-002 with blocks=['AUTH-004'], and AUTH-003 and AUTH-004 with no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            (
                "AUTH-001",
                json!({ "relationships": { "blocks": ["AUTH-002", "AUTH-003"] } }),
            ),
            (
                "AUTH-002",
                json!({ "relationships": { "blocks": ["AUTH-004"] } }),
            ),
            ("AUTH-003", json!({})),
            ("AUTH-004", json!({})),
        ]),
    );

    // @step When I dispatch dependencies with workUnitId='AUTH-001' and graph=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "graph": true }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the data field equals the TS `traverse` output: every visited node prints its id (root AND each recursed child), and recursion increases indent by two levels
    // Parity with src/commands/dependencies.ts:837-856 — the TS traverse pushes
    // `${prefix}${id}` for EVERY visited node and recurses with `indent + 2`.
    let expected = "AUTH-001\n  blocks → AUTH-002\n    AUTH-002\n      blocks → AUTH-004\n        AUTH-004\n  blocks → AUTH-003\n    AUTH-003";
    assert_eq!(result.data, expected, "got:\n{}", result.data);
}

#[test]
fn relationship_values_fall_back_to_legacy_top_level_fields_when_relationships_object_is_absent() {
    // Scenario: Relationship values fall back to legacy top-level fields when relationships object is absent

    // @step Given spec/work-units.json contains LEGACY-001 with top-level dependsOn=['LEGACY-002'] and no relationships object
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("LEGACY-001", json!({ "dependsOn": ["LEGACY-002"] })),
            ("LEGACY-002", json!({})),
        ]),
    );

    // @step When I dispatch dependencies with workUnitId='LEGACY-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "LEGACY-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the data field equals "Dependencies for LEGACY-001:\n  Depends on: LEGACY-002\n"
    assert_eq!(
        result.data, "Dependencies for LEGACY-001:\n  Depends on: LEGACY-002\n",
        "got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_for_a_missing_work_unit_returns_a_structured_does_not_exist_error() {
    // Scenario: Dispatch dependencies for a missing work unit returns a structured does-not-exist error

    // @step Given spec/work-units.json contains AUTH-001 only
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("AUTH-001", json!({}))]));

    // @step When I dispatch dependencies with workUnitId='INVALID-999'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "INVALID-999" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'does not exist'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("does not exist"),
        "error message missing 'does not exist' substring: {msg}"
    );
}
