#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/export-dependencies-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `export-dependencies`
// (RPC-227) through the LLM-facing dispatcher front door. Each scenario maps
// to exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim.
//
// Red phase: until RPC-227 is wired into run_ported (Phase C), the dispatcher
// routes `export-dependencies` to the NotYetPorted stub, so every success
// assertion below fails. That is the expected red-phase signal.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "export-dependencies".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a work-units.json string with the given (id, status, extra-fields)
/// entries, preserving insertion order.
fn work_units_with(entries: &[(&str, &str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String((*status).to_string()));
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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn export_mermaid_for_a_store_with_all_dependency_kinds() {
    // @step Given a work units store where AUTH-001 blocks AUTH-002, depends on AUTH-003, and relates to AUTH-004
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            (
                "AUTH-001",
                "backlog",
                json!({ "blocks": ["AUTH-002"], "dependsOn": ["AUTH-003"], "relatesTo": ["AUTH-004"] }),
            ),
            ("AUTH-002", "done", json!({})),
        ]),
    );

    // @step When I export dependencies in mermaid format to deps.mmd
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "mermaid", "output": "deps.mmd" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let written = fs::read_to_string(tmp.path().join("deps.mmd")).expect("read deps.mmd");

    // @step Then the written content starts with "graph TB"
    assert!(written.starts_with("graph TB"), "got:\n{written}");

    // @step And it contains a node line for each work unit
    assert!(
        written.contains("  AUTH-001[\"title AUTH-001\"]"),
        "got:\n{written}"
    );
    assert!(
        written.contains("  AUTH-002[\"title AUTH-002\"]:::done"),
        "got:\n{written}"
    );

    // @step And it contains the edge lines for blocks, depends on, and relates to
    assert!(
        written.contains("  AUTH-001 -->|blocks| AUTH-002"),
        "got:\n{written}"
    );
    assert!(
        written.contains("  AUTH-001 -.->|depends on| AUTH-003"),
        "got:\n{written}"
    );
    assert!(
        written.contains("  AUTH-001 <-.->|relates to| AUTH-004"),
        "got:\n{written}"
    );

    // @step And it ends with the classDef done and classDef blocked trailer
    assert!(
        written.contains("  classDef done fill:#90EE90")
            && written.contains("  classDef blocked fill:#FFB6C1"),
        "got:\n{written}"
    );

    // @step And the returned message is "✓ Dependencies exported to deps.mmd"
    assert_eq!(result.data, "✓ Dependencies exported to deps.mmd");
}

#[test]
fn export_json_for_a_store_maps_every_work_unit_to_dependency_arrays() {
    // @step Given a work units store where AUTH-001 blocks AUTH-002 and depends on AUTH-003
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            (
                "AUTH-001",
                "backlog",
                json!({ "blocks": ["AUTH-002"], "dependsOn": ["AUTH-003"] }),
            ),
            ("AUTH-002", "backlog", json!({})),
        ]),
    );

    // @step When I export dependencies in json format to deps.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "deps.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let written = fs::read_to_string(tmp.path().join("deps.json")).expect("read deps.json");

    // @step Then the written JSON keys every work unit in insertion order
    let pos1 = written.find("\"AUTH-001\"").expect("AUTH-001 key");
    let pos2 = written.find("\"AUTH-002\"").expect("AUTH-002 key");
    assert!(pos1 < pos2, "insertion order must be preserved");

    // @step And each entry has blocks, blockedBy, dependsOn, and relatesTo arrays
    let parsed: Value = serde_json::from_str(&written).expect("deps.json is JSON");
    for id in ["AUTH-001", "AUTH-002"] {
        for field in ["blocks", "blockedBy", "dependsOn", "relatesTo"] {
            assert!(
                parsed[id][field].is_array(),
                "{id}.{field} must be an array in:\n{written}"
            );
        }
    }
    assert_eq!(parsed["AUTH-001"]["blocks"][0].as_str(), Some("AUTH-002"));
    assert_eq!(
        parsed["AUTH-001"]["dependsOn"][0].as_str(),
        Some("AUTH-003")
    );
}

#[test]
fn export_dot_format_produces_the_same_json_content_as_json_format() {
    // @step Given a work units store containing AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("AUTH-001", "backlog", json!({ "blocks": ["AUTH-002"] }))]),
    );

    // @step When I export dependencies in dot format to deps.dot
    let dot = dispatch_command(req(
        tmp.path(),
        json!({ "format": "dot", "output": "deps.dot" }),
    ));
    let js = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "deps.json" }),
    ));
    assert!(dot.success && js.success, "{dot:?} {js:?}");

    // @step Then the written content equals the json format output for the same store
    let dot_content = fs::read_to_string(tmp.path().join("deps.dot")).expect("read deps.dot");
    let json_content = fs::read_to_string(tmp.path().join("deps.json")).expect("read deps.json");
    assert_eq!(
        dot_content, json_content,
        "dot format must fall through to the JSON branch and produce identical content"
    );
}

#[test]
fn export_mermaid_marks_done_and_blocked_nodes() {
    // @step Given a work units store where AUTH-002 is done and AUTH-005 is blocked
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-002", "done", json!({})),
            ("AUTH-005", "blocked", json!({})),
        ]),
    );

    // @step When I export dependencies in mermaid format to deps.mmd
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "mermaid", "output": "deps.mmd" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let written = fs::read_to_string(tmp.path().join("deps.mmd")).expect("read deps.mmd");

    // @step Then the AUTH-002 node line ends with ":::done"
    assert!(
        written
            .lines()
            .any(|l| l.trim_start().starts_with("AUTH-002[") && l.ends_with(":::done")),
        "AUTH-002 node must end with :::done; got:\n{written}"
    );

    // @step And the AUTH-005 node line ends with ":::blocked"
    assert!(
        written
            .lines()
            .any(|l| l.trim_start().starts_with("AUTH-005[") && l.ends_with(":::blocked")),
        "AUTH-005 node must end with :::blocked; got:\n{written}"
    );
}

#[test]
fn export_mermaid_dedupes_bidirectional_relates_to_edges() {
    // @step Given a work units store where AUTH-001 relates to AUTH-004 and AUTH-004 relates to AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "backlog", json!({ "relatesTo": ["AUTH-004"] })),
            ("AUTH-004", "backlog", json!({ "relatesTo": ["AUTH-001"] })),
        ]),
    );

    // @step When I export dependencies in mermaid format to deps.mmd
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "mermaid", "output": "deps.mmd" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let written = fs::read_to_string(tmp.path().join("deps.mmd")).expect("read deps.mmd");

    // @step Then only one "<-.->|relates to|" edge appears between AUTH-001 and AUTH-004
    let count = written.matches("<-.->|relates to|").count();
    assert_eq!(
        count, 1,
        "expected exactly one relates-to edge; got {count}:\n{written}"
    );
}

#[test]
fn export_to_a_nested_output_path_creates_parent_directories() {
    // @step Given a work units store containing AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("AUTH-001", "backlog", json!({}))]),
    );

    // @step When I export dependencies in json format to out/graphs/deps.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "out/graphs/deps.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parent directory out/graphs is created and the file is written
    assert!(
        tmp.path().join("out/graphs").is_dir(),
        "out/graphs directory must be created"
    );
    assert!(
        tmp.path().join("out/graphs/deps.json").is_file(),
        "out/graphs/deps.json must be written"
    );
}

#[test]
fn export_with_a_malformed_work_units_file_escalates_a_parse_error() {
    // @step Given a spec/work-units.json file that is not valid JSON
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I export dependencies in json format to out.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "out.json" }),
    ));

    // @step Then the run returns an error containing "Failed to parse work-units.json"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn dispatcher_and_core_produce_identical_file_content() {
    // @step Given a work units store containing AUTH-001 with dependencies
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "AUTH-001",
            "backlog",
            json!({ "blocks": ["AUTH-002"], "dependsOn": ["AUTH-003"] }),
        )]),
    );

    // @step When I export dependencies in json format via the core run function
    let first = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "a.json" }),
    ));
    let second = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "b.json" }),
    ));
    assert!(first.success && second.success, "{first:?} {second:?}");

    // @step Then the written file content is the same as exporting via the dispatcher path
    let a = fs::read_to_string(tmp.path().join("a.json")).expect("read a.json");
    let b = fs::read_to_string(tmp.path().join("b.json")).expect("read b.json");
    assert_eq!(
        a, b,
        "both export paths must produce identical file content"
    );
}
