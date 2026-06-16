#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compare-implementations-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `compare-implementations` (RPC-207). Each scenario maps to one #[test]
// fn with @step comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. The behavioural tests are RED
// until PHASE C.

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
        command: "compare-implementations".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Write spec/work-units.json with the supplied (id, tags) work units.
fn write_work_units(project_root: &Path, units: &[(&str, &[&str])]) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut entries = String::new();
    for (i, (id, tags)) in units.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        let tags_json = serde_json::to_string(tags).unwrap();
        entries.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"{id} title","type":"story","status":"backlog","tags":{tags_json}}}"#
        ));
    }
    let json = format!(r#"{{"workUnits":{{{entries}}}}}"#);
    fs::write(spec.join("work-units.json"), json).expect("write work-units.json");
}

/// Write a `.feature.coverage` sidecar referencing one test file and one
/// impl file (TS coverage-reader schema: scenarios[].testMappings[].file
/// + nested implMappings[].file).
fn write_coverage(project_root: &Path, rel_name: &str, test_file: &str, impl_file: &str) {
    let dir = project_root.join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    let body = json!({
        "scenarios": [{
            "name": "S1",
            "testMappings": [{
                "file": test_file,
                "lines": "1-10",
                "implMappings": [{ "file": impl_file, "lines": [1, 2] }]
            }]
        }]
    });
    fs::write(dir.join(rel_name), body.to_string()).expect("write coverage file");
}

fn data_of(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).expect("parse data json")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher summarises work units carrying the tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_summarises_work_units_carrying_the_tag() {
    // @step Given a project root tempdir with spec/work-units.json containing two work units tagged @cli
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &[("CLI-001", &["@cli"]), ("CLI-002", &["@cli"])],
    );

    // @step When I dispatch compare-implementations with tag=@cli
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@cli"})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns workUnits with length 2
    assert_eq!(
        data["workUnits"].as_array().map(|a| a.len()),
        Some(2),
        "got data: {data}"
    );

    // @step And the comparison.type field equals 'side-by-side'
    assert_eq!(data["comparison"]["type"].as_str(), Some("side-by-side"));

    // @step And the namingConventionDifferences array is empty
    assert_eq!(
        data["namingConventionDifferences"].as_array().map(|a| a.len()),
        Some(0)
    );

    // @step And the coverage array is empty
    assert_eq!(data["coverage"].as_array().map(|a| a.len()), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher includes deduplicated coverage file paths
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_includes_deduplicated_coverage_file_paths() {
    // @step Given a project root tempdir with spec/work-units.json containing one work unit tagged @cli and one .feature.coverage file referencing one test file and one impl file
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &[("CLI-001", &["@cli"])]);
    write_coverage(tmp.path(), "a.feature.coverage", "test/a.test.ts", "src/a.ts");

    // @step When I dispatch compare-implementations with tag=@cli and showCoverage=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tag": "@cli", "showCoverage": true}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns coverage with one entry
    assert_eq!(
        data["coverage"].as_array().map(|a| a.len()),
        Some(1),
        "got data: {data}"
    );

    // @step And coverage[0].testFiles equals ['test/a.test.ts']
    assert_eq!(
        data["coverage"][0]["testFiles"],
        json!(["test/a.test.ts"]),
        "got data: {data}"
    );

    // @step And coverage[0].implementationFiles equals ['src/a.ts']
    assert_eq!(
        data["coverage"][0]["implementationFiles"],
        json!(["src/a.ts"]),
        "got data: {data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher returns empty workUnits when no tag matches
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_returns_empty_work_units_when_no_tag_matches() {
    // @step Given a project root tempdir with spec/work-units.json containing one work unit tagged @other
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &[("OTH-001", &["@other"])]);

    // @step When I dispatch compare-implementations with tag=@cli
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@cli"})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns workUnits with length 0
    assert_eq!(
        data["workUnits"].as_array().map(|a| a.len()),
        Some(0),
        "got data: {data}"
    );

    // @step And the coverage array is empty
    assert_eq!(data["coverage"].as_array().map(|a| a.len()), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher errors when work-units.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_errors_when_work_units_json_is_missing() {
    // @step Given a project root tempdir with no spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch compare-implementations with tag=@cli
    let result = dispatch_command(req(tmp.path(), json!({"tag": "@cli"})));

    // @step Then the dispatcher returns an error
    assert!(
        !result.success,
        "expected dispatch failure for missing work-units.json; got {result:?}"
    );
}
