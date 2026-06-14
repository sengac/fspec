#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
// Feature: spec/features/search-implementation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `search-implementation`
// (RPC-296). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the dispatcher arm still calls the 1-arg stub which returns
// FspecCoreError::NotYetPorted, so these tests COMPILE and FAIL at runtime
// until the Phase C port lands.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: "search-implementation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// One coverage sidecar for `user-login` with a single test mapping whose
/// implMappings reference `impl_rel`. featureName is derived from the file
/// name (`user-login`) so the uppercased work-unit id is `USER-LOGIN`.
fn write_single_impl_sidecar(root: &Path, impl_rel: &str) {
    let body = format!(
        r#"{{
  "scenarios": [
    {{
      "name": "Login",
      "testMappings": [
        {{
          "file": "test/login.test.ts",
          "lines": "1-10",
          "implMappings": [
            {{ "file": "{impl_rel}", "lines": [1, 2, 3] }}
          ]
        }}
      ]
    }}
  ],
  "stats": {{
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["test/login.test.ts"],
    "implFiles": ["{impl_rel}"],
    "totalLinesCovered": 0
  }}
}}"#
    );
    write_file(root, "spec/features/user-login.feature.coverage", &body);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Function found in a linked implementation file
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn function_found_in_a_linked_implementation_file() {
    // @step Given a temp project root has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "src/config.ts", "export function loadConfig() {}\n");
    write_single_impl_sidecar(tmp.path(), "src/config.ts");

    // @step When I dispatch search-implementation with function="loadConfig"
    let result = req(tmp.path(), json!({"function": "loadConfig"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the files array contains an entry whose filePath is that implementation file
    let files = v["files"].as_array().expect("files array");
    let entry = files
        .iter()
        .find(|f| f["filePath"].as_str() == Some("src/config.ts"))
        .expect("entry for src/config.ts");

    // @step And that entry's workUnits array carries the uppercased feature name
    let work_units = entry["workUnits"].as_array().expect("workUnits array");
    assert!(
        work_units
            .iter()
            .any(|w| w["workUnitId"].as_str() == Some("USER-LOGIN")),
        "workUnits must carry uppercased feature name USER-LOGIN; got {work_units:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Function found in zero files returns an empty files array
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn function_found_in_zero_files_returns_an_empty_files_array() {
    // @step Given a temp project root has a coverage sidecar whose implMappings reference an on-disk file NOT containing "zzzNope"
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "src/config.ts", "export function loadConfig() {}\n");
    write_single_impl_sidecar(tmp.path(), "src/config.ts");

    // @step When I dispatch search-implementation with function="zzzNope"
    let result = req(tmp.path(), json!({"function": "zzzNope"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the files array is empty
    assert!(v["files"].as_array().expect("files array").is_empty());
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Multiple impl mappings are counted in searchedFiles
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn multiple_impl_mappings_are_counted_in_searched_files() {
    // @step Given a temp project root has a coverage sidecar referencing two implMappings file paths
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        {
          "file": "test/login.test.ts",
          "lines": "1-10",
          "implMappings": [
            { "file": "src/a.ts", "lines": [1, 2] },
            { "file": "src/b.ts", "lines": [3, 4] }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["test/login.test.ts"],
    "implFiles": ["src/a.ts", "src/b.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/user-login.feature.coverage", body);

    // @step When I dispatch search-implementation with function="anything"
    let result = req(tmp.path(), json!({"function": "anything"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the searchedFiles field equals 2
    assert_eq!(v["searchedFiles"].as_u64(), Some(2));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Unreadable implementation files are skipped without error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn unreadable_implementation_files_are_skipped_without_error() {
    // @step Given a temp project root has a coverage sidecar referencing an implMappings path that does not exist on disk
    let tmp = TempDir::new().expect("tempdir");
    write_single_impl_sidecar(tmp.path(), "src/does-not-exist.ts");

    // @step When I dispatch search-implementation with function="loadConfig"
    let result = req(tmp.path(), json!({"function": "loadConfig"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the files array is empty
    assert!(v["files"].as_array().expect("files array").is_empty());
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Missing spec/features directory yields zero searched files
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn missing_spec_features_directory_yields_zero_searched_files() {
    // @step Given a temp project root with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch search-implementation with function="anything"
    let result = req(tmp.path(), json!({"function": "anything"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the searchedFiles field equals 0
    assert_eq!(v["searchedFiles"].as_u64(), Some(0));

    // @step And the files array is empty
    assert!(v["files"].as_array().expect("files array").is_empty());
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Shared infrastructure module is registered for search-implementation
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn shared_infrastructure_module_is_registered_for_search_implementation() {
    // @step Given the codelet/fspec-core crate is built
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "src/config.ts", "export function loadConfig() {}\n");
    write_single_impl_sidecar(tmp.path(), "src/config.ts");

    // @step When I inspect codelet/fspec-core/src/commands/search_implementation.rs
    let result = req(tmp.path(), json!({"function": "loadConfig"}));

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-296"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes search-implementation to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when args are valid; got {result:?}"
    );
}
