//! Feature: spec/features/show-test-patterns-rust-port.feature
//!
//! Dispatcher-level tests for `show-test-patterns` (RPC-307), moved out of
//! rust/fspec/tests/cli_show_test_patterns.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

// ───────── Helpers ─────────

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "show-test-patterns".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_work_units(cwd: &Path, raw: &str) {
    write_file(cwd, "spec/work-units.json", raw);
}

// ─── Sample work-unit bodies ───

fn wus_two_cli_one_untagged() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"},
    "WU-002": {"id":"WU-002","title":"b","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"},
    "WU-003": {"id":"WU-003","title":"c","type":"story","status":"backlog","tags":[],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001","WU-002","WU-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn wus_two_untagged() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@auth"],"createdAt":"x","updatedAt":"x"},
    "WU-002": {"id":"WU-002","title":"b","type":"story","status":"backlog","tags":["@db"],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001","WU-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn wus_one_cli() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn cov_two_files_three_unique_tests(ws: &Path) {
    // Two coverage files; together reference three unique testMappings.filePath strings.
    write_file(ws, "spec/features/login.feature", "Feature: Login\n");
    write_file(ws, "spec/features/auth.feature", "Feature: Auth\n");
    let cov_login = r#"{
  "featureName": "Login",
  "filePath": "spec/features/login.feature",
  "scenarios": [
    {"name":"S1","testMappings":[{"filePath":"test/a.test.ts","testLines":"1-10"},{"filePath":"test/b.test.ts","testLines":"1-10"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    let cov_auth = r#"{
  "featureName": "Auth",
  "filePath": "spec/features/auth.feature",
  "scenarios": [
    {"name":"S2","testMappings":[{"filePath":"test/b.test.ts","testLines":"5-10"},{"filePath":"test/c.test.ts","testLines":"1-5"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    write_file(ws, "spec/features/login.feature.coverage", cov_login);
    write_file(ws, "spec/features/auth.feature.coverage", cov_auth);
}

fn cov_one_file_with_paths(ws: &Path) {
    write_file(ws, "spec/features/login.feature", "Feature: Login\n");
    let cov = r#"{
  "featureName": "Login",
  "filePath": "spec/features/login.feature",
  "scenarios": [
    {"name":"S1","testMappings":[{"filePath":"test/x.test.ts","testLines":"1-10"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    write_file(ws, "spec/features/login.feature.coverage", cov);
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 1: Missing tag argument surfaces error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_missing_tag_argument_surfaces_invalid_args_error() {
    // @step Given a temp project root contains a valid spec/work-units.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with no tag argument
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'tag'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("tag"),
        "error must mention tag; got: {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 2: Tag matches zero work units returns empty
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_matches_zero_work_units_returns_empty_data() {
    // @step Given a temp project root contains spec/work-units.json with two work units neither tagged @missing
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_untagged());

    // @step When I dispatch show-test-patterns with tag='@missing'
    let result = dispatch(ws.path(), r#"{"tag":"@missing"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.workUnits array is empty
    assert_eq!(parsed["workUnits"].as_array().unwrap().len(), 0);

    // @step And the data.patterns array is empty
    assert_eq!(parsed["patterns"].as_array().unwrap().len(), 0);

    // @step And the data.testFiles array is empty
    assert_eq!(parsed["testFiles"].as_array().unwrap().len(), 0);

    // @step And the data.format equals 'table'
    assert_eq!(parsed["format"].as_str(), Some("table"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 3: Tag matches work units returns their tags
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_matches_work_units_returns_their_tags_arrays() {
    // @step Given a temp project root contains spec/work-units.json with two work units tagged @cli and one untagged
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_cli_one_untagged());

    // @step When I dispatch show-test-patterns with tag='@cli'
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.workUnits array has 2 elements
    let wus = parsed["workUnits"].as_array().expect("workUnits array");
    assert_eq!(wus.len(), 2);

    // @step And every workUnits[i].tags array contains '@cli'
    for wu in wus {
        let tags = wu["tags"].as_array().expect("tags array");
        assert!(tags.iter().any(|t| t.as_str() == Some("@cli")));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 4: includeCoverage true reads & dedupes
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_include_coverage_true_reads_and_dedupes_test_files() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli and two .feature.coverage files referencing three unique testMappings file paths
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    cov_two_files_three_unique_tests(ws.path());

    // @step When I dispatch show-test-patterns with tag='@cli' and includeCoverage=true
    let result = dispatch(ws.path(), r#"{"tag":"@cli","includeCoverage":true}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.testFiles array has 3 unique elements
    let tf = parsed["testFiles"].as_array().expect("testFiles array");
    assert_eq!(tf.len(), 3, "expected 3 unique test files; got {tf:?}");
    let mut seen: Vec<&str> = tf.iter().filter_map(|v| v.as_str()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 3, "values must be unique; got {seen:?}");
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 5: includeCoverage false leaves testFiles empty
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_include_coverage_false_leaves_test_files_empty() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli and one .feature.coverage file referencing testMappings paths
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    cov_one_file_with_paths(ws.path());

    // @step When I dispatch show-test-patterns with tag='@cli' and no includeCoverage flag
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.testFiles array is empty
    assert_eq!(parsed["testFiles"].as_array().unwrap().len(), 0);
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 6: json format sets data.format to 'json'
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_json_format_sets_data_format_to_json() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with tag='@cli' and json=true
    let result = dispatch(ws.path(), r#"{"tag":"@cli","json":true}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.format equals 'json'
    assert_eq!(parsed["format"].as_str(), Some("json"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 7: Default format flag yields 'table'
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_default_format_flag_yields_table_format() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with tag='@cli' and no json flag
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.format equals 'table'
    assert_eq!(parsed["format"].as_str(), Some("table"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 8: Shared infrastructure module is registered
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_shared_infrastructure_module_is_registered() {
    // @step Given the rust/fspec-core crate is built

    // @step When I inspect rust/fspec-core/src/commands/show_test_patterns.rs
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-307"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes show-test-patterns to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when args are valid; got {result:?}"
    );
}
