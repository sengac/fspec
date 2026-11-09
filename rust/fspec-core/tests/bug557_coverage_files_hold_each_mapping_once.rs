//! BUG-557 — coverage files hold each mapping once.
//!
//! An exact duplicate is a second test mapping of one scenario with the same
//! file and lines text, or a second implementation mapping of one test
//! mapping with the same file and line list. link-coverage merges instead of
//! duplicating, fspec validate reports exact duplicates, and a one-off
//! script merges the existing ones.
//!
//! Feature: spec/features/coverage-files-hold-each-mapping-once.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";
const TEST: &str = "tests/login.rs";
/// Steps on lines 2-4 and again on lines 8-10.
const TEST_SOURCE: &str = "fn log_in() {\n    // @step Given a user\n    // @step When they log in\n    // @step Then they see the board\n}\n\nfn log_in_again() {\n    // @step Given a user\n    // @step When they log in\n    // @step Then they see the board\n}\n";

fn project(mappings: Value) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |relative: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/login.feature", FEATURE)?;
    write(TEST, TEST_SOURCE)?;
    write(
        "src/login.rs",
        &(1..=5)
            .map(|n| format!("// impl {n}\n"))
            .collect::<String>(),
    )?;
    write(
        "spec/features/login.feature.coverage",
        &serde_json::to_string_pretty(&json!({
            "scenarios": [{"name": "Log in", "testMappings": mappings}],
            "stats": {"totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
                "testFiles": [], "implFiles": [], "totalLinesCovered": 0}
        }))?,
    )?;
    Ok(root)
}

fn dispatch(root: &Path, command: &str, args: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("{command} failed: {:?}", result.error).into());
    }
    Ok(serde_json::from_str(&result.data).unwrap_or(Value::Null))
}

fn mappings(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(root.join("spec/features/login.feature.coverage"))?;
    let coverage: Value = serde_json::from_str(&raw)?;
    Ok(coverage["scenarios"][0]["testMappings"].clone())
}

fn validate(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let report = dispatch(root, "validate", json!({}))?;
    let output = report["output"].as_str().unwrap_or_default().to_string();
    Ok((report, output))
}

#[test]
fn linking_an_already_mapped_test_range_merges_into_the_existing_mapping() -> TestResult {
    // @step Given scenario "Log in" maps tests/login.rs:1-5
    let root = project(json!([{"file": TEST, "lines": "1-5"}]))?;
    // @step When link-coverage links tests/login.rs:1-5 again with the implementation src/login.rs lines 2-3
    dispatch(
        root.path(),
        "link-coverage",
        json!({"featureName": "login", "scenario": "Log in", "testFile": TEST,
            "testLines": "1-5", "implFile": "src/login.rs", "implLines": "2,3"}),
    )?;
    // @step Then the scenario still has one tests/login.rs:1-5 mapping, and it carries src/login.rs lines 2-3
    assert_eq!(
        mappings(root.path())?,
        json!([{"file": TEST, "lines": "1-5",
            "implMappings": [{"file": "src/login.rs", "lines": [2, 3],
                "firstLine": "// impl 2", "lastLine": "// impl 3"}]}])
    );
    Ok(())
}

#[test]
fn validate_reports_an_exact_duplicate_test_mapping() -> TestResult {
    // @step Given scenario "Log in" of login.feature.coverage maps tests/login.rs:1-5 twice
    let root = project(json!([
        {"file": TEST, "lines": "1-5"},
        {"file": TEST, "lines": "1-5"}
    ]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports the duplicate tests/login.rs:1-5 mapping of scenario "Log in" in spec/features/login.feature.coverage and exits 1
    assert!(
        output.contains(
            "✗ spec/features/login.feature.coverage maps tests/login.rs:1-5 more than once in scenario \"Log in\""
        ),
        "{output}"
    );
    assert_eq!(report["exitCode"], 1, "{output}");
    Ok(())
}

#[test]
fn validate_reports_an_exact_duplicate_implementation_mapping() -> TestResult {
    // @step Given a test mapping of scenario "Log in" carries the implementation src/login.rs lines 2-3 twice
    let root = project(json!([{"file": TEST, "lines": "1-5", "implMappings": [
        {"file": "src/login.rs", "lines": [2, 3]},
        {"file": "src/login.rs", "lines": [2, 3]}
    ]}]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports the duplicate src/login.rs implementation mapping and exits 1
    assert!(
        output.contains(
            "✗ spec/features/login.feature.coverage maps the implementation src/login.rs [2, 3] more than once under tests/login.rs:1-5 in scenario \"Log in\""
        ),
        "{output}"
    );
    assert_eq!(report["exitCode"], 1, "{output}");
    Ok(())
}

#[test]
fn same_file_mappings_with_different_ranges_pass() -> TestResult {
    // @step Given scenario "Log in" maps tests/login.rs:1-5 and tests/login.rs:7-11
    let root = project(json!([
        {"file": TEST, "lines": "1-5"},
        {"file": TEST, "lines": "7-11"}
    ]))?;
    // @step When fspec validate runs
    let (report, output) = validate(root.path())?;
    // @step Then it reports no duplicate mapping and exits 0
    assert!(!output.contains("more than once"), "{output}");
    assert_eq!(report["exitCode"], 0, "{output}");
    Ok(())
}

#[test]
fn the_cleanup_script_merges_existing_duplicates() -> TestResult {
    // @step Given two tests/login.rs:1-5 mappings of one scenario carry src/a.rs [2] and src/a.rs [3] with src/b.rs [1]
    let root = project(json!([
        {"file": TEST, "lines": "1-5", "implMappings": [{"file": "src/a.rs", "lines": [2]}]},
        {"file": TEST, "lines": "7-11"},
        {"file": TEST, "lines": "1-5", "implMappings": [
            {"file": "src/a.rs", "lines": [3]}, {"file": "src/b.rs", "lines": [1]}
        ]}
    ]))?;
    let coverage = root.path().join("spec/features/login.feature.coverage");
    let mut expected: Value = serde_json::from_str(&std::fs::read_to_string(&coverage)?)?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/merge-duplicate-coverage-mappings.py")
        .canonicalize()?;

    // @step When scripts/merge-duplicate-coverage-mappings.py runs in the project
    let status = std::process::Command::new("python3")
        .arg("-I")
        .arg(&script)
        .current_dir(root.path())
        .status()?;
    assert!(status.success(), "the cleanup script failed: {status}");

    // @step Then one tests/login.rs:1-5 mapping remains, carrying src/a.rs [2, 3] and src/b.rs [1], and nothing else in the file changes
    expected["scenarios"][0]["testMappings"] = json!([
        {"file": TEST, "lines": "1-5", "implMappings": [
            {"file": "src/a.rs", "lines": [2, 3]}, {"file": "src/b.rs", "lines": [1]}
        ]},
        {"file": TEST, "lines": "7-11"}
    ]);
    let merged: Value = serde_json::from_str(&std::fs::read_to_string(&coverage)?)?;
    assert_eq!(merged, expected);
    Ok(())
}
