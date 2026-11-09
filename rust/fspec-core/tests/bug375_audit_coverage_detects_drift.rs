//! BUG-375 — fspec audit detects drifted test mappings.
//!
//! Feature: spec/features/audit-coverage-detects-drift.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const TEST_FILE: &str = "\
// helper line 1
// helper line 2
fn log_in() {
    // @step Given a user
    // @step When they log in
    // @step Then they see the board
}
";

fn project(lines: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/login.feature", FEATURE)?;
    write("tests/login_test.rs", TEST_FILE)?;
    write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": [
            {"file": "tests/login_test.rs", "lines": lines, "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    Ok(root)
}

fn audit(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["login"], "area": ["coverage"]}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

#[test]
fn a_test_mapping_that_lost_its_steps_is_reported_as_drifted() -> TestResult {
    // @step Given a feature whose coverage maps a scenario to lines that do not hold its @step comments
    let root = project("1-4")?;
    // @step When the coverage is audited
    let report = audit(root.path())?;
    // @step Then the audit fails naming the mapping and the missing step
    let output = report["output"].as_str().unwrap_or_default();
    assert_eq!(report["exitCode"], 1, "{output}");
    assert!(output.contains("tests/login_test.rs:1-4"), "{output}");
    assert!(output.contains("When they log in"), "{output}");
    Ok(())
}

#[test]
fn a_test_mapping_that_holds_its_steps_passes_the_audit() -> TestResult {
    // @step Given a feature whose coverage maps a scenario to the lines holding its @step comments
    let root = project("3-7")?;
    // @step When the coverage is audited
    let report = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    let output = report["output"].as_str().unwrap_or_default();
    assert_eq!(report["exitCode"], 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}
