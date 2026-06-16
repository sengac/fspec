#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/audit-coverage-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `audit-coverage`
// (RPC-197). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, feature: &str) -> DispatchRequest {
    DispatchRequest {
        command: "audit-coverage".to_string(),
        args_json: json!({ "featureName": feature }).to_string(),
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

fn write_coverage(root: &Path, feature: &str, scenarios_json: &str) {
    write_file(
        root,
        &format!("spec/features/{feature}.feature.coverage"),
        &format!("{{\n  \"scenarios\": {scenarios_json}\n}}"),
    );
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

// ---------- scenarios ----------

#[test]
fn reports_all_files_present_when_every_referenced_file_exists() {
    // Scenario: Reports all files present when every referenced file exists

    // @step Given a coverage file user-login.feature.coverage referencing three files that all exist on disk
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/a.test.ts", "// a\n");
    write_file(tmp.path(), "src/a.ts", "// a impl\n");
    write_file(tmp.path(), "tests/b.test.ts", "// b\n");
    write_coverage(
        tmp.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "tests/a.test.ts", "lines": "1-5", "implMappings": [ { "file": "src/a.ts", "lines": "1-3" } ] },
      { "file": "tests/b.test.ts", "lines": "1-5", "implMappings": [] }
    ] }
  ]"#,
    );

    // @step When I dispatch the audit-coverage command for feature 'user-login' against that project root
    let result = dispatch_command(req(tmp.path(), "user-login"));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the output contains the substring '✅ All files found (3/3)'
    assert!(
        data["output"].as_str().unwrap().contains("✅ All files found (3/3)"),
        "output: {}",
        data["output"]
    );

    // @step Then the output contains the substring 'All mappings valid'
    assert!(data["output"].as_str().unwrap().contains("All mappings valid"));

    // @step Then the envelope exitCode is 0
    assert_eq!(data["exitCode"].as_i64(), Some(0));
}

#[test]
fn detects_a_missing_test_file_and_recommends_removing_the_mapping() {
    // Scenario: Detects a missing test file and recommends removing the mapping

    // @step Given a coverage file user-login.feature.coverage mapping to the test file 'src/__tests__/deleted.test.ts' which does not exist
    let tmp = TempDir::new().expect("tempdir");
    write_coverage(
        tmp.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "src/__tests__/deleted.test.ts", "lines": "1-5", "implMappings": [] }
    ] }
  ]"#,
    );

    // @step When I dispatch the audit-coverage command for feature 'user-login' against that project root
    let result = dispatch_command(req(tmp.path(), "user-login"));
    let data = parse_data(&result);

    // @step Then the output contains the substring '❌ Test file not found: src/__tests__/deleted.test.ts'
    assert!(
        data["output"]
            .as_str()
            .unwrap()
            .contains("❌ Test file not found: src/__tests__/deleted.test.ts"),
        "output: {}",
        data["output"]
    );

    // @step Then the output contains the substring 'Recommendation: Remove this mapping or restore the deleted file'
    assert!(data["output"]
        .as_str()
        .unwrap()
        .contains("Recommendation: Remove this mapping or restore the deleted file"));

    // @step Then the envelope exitCode is 1
    assert_eq!(data["exitCode"].as_i64(), Some(1));
}

#[test]
fn detects_a_missing_implementation_file() {
    // Scenario: Detects a missing implementation file

    // @step Given a coverage file user-login.feature.coverage whose test file exists but maps to the implementation file 'src/auth/deleted.ts' which does not exist
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/a.test.ts", "// a\n");
    write_coverage(
        tmp.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "tests/a.test.ts", "lines": "1-5", "implMappings": [ { "file": "src/auth/deleted.ts", "lines": "1-3" } ] }
    ] }
  ]"#,
    );

    // @step When I dispatch the audit-coverage command for feature 'user-login' against that project root
    let result = dispatch_command(req(tmp.path(), "user-login"));
    let data = parse_data(&result);

    // @step Then the output contains the substring '❌ Implementation file not found: src/auth/deleted.ts'
    assert!(
        data["output"]
            .as_str()
            .unwrap()
            .contains("❌ Implementation file not found: src/auth/deleted.ts"),
        "output: {}",
        data["output"]
    );

    // @step Then the envelope exitCode is 1
    assert_eq!(data["exitCode"].as_i64(), Some(1));
}

#[test]
fn reports_a_missing_coverage_file_with_the_full_path() {
    // Scenario: Reports a missing coverage file with the full path

    // @step Given a project root with no spec/features/user-login.feature.coverage file
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the audit-coverage command for feature 'user-login' against that project root
    let result = dispatch_command(req(tmp.path(), "user-login"));
    let data = parse_data(&result);

    // @step Then the output contains the substring '✗ Coverage file not found:'
    assert!(
        data["output"].as_str().unwrap().contains("✗ Coverage file not found:"),
        "output: {}",
        data["output"]
    );

    // @step Then the envelope exitCode is 1
    assert_eq!(data["exitCode"].as_i64(), Some(1));
}
