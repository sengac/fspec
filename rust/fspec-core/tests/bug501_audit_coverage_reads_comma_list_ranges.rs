//! BUG-501 — fspec audit reads comma-list line ranges.
//!
//! Feature: spec/features/audit-coverage-reads-comma-list-line-ranges.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

/// The @step comments are on lines 4 to 6.
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

fn audit(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["login"], "area": ["coverage"]}).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let report: Value = serde_json::from_str(&result.data)?;
    let output = report["output"].as_str().unwrap_or_default().to_string();
    Ok((report["exitCode"].clone(), output))
}

#[test]
fn a_comma_list_mapping_whose_parts_hold_the_steps_passes_the_audit() -> TestResult {
    // @step Given scenario "Log in" maps test lines "4-4,5-6" which together hold its @step comments
    let root = project("4-4,5-6")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn a_comma_list_mapping_that_lacks_a_step_is_reported_as_drifted() -> TestResult {
    // @step Given scenario "Log in" maps test lines "4-4,1-2" which lack its When step
    let root = project("4-4,1-2")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails naming the missing When step
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("no @step comment for 'When they log in'"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_comma_list_with_a_part_that_is_not_a_range_is_unreadable() -> TestResult {
    // @step Given scenario "Log in" maps test lines "4-6,x"
    let root = project("4-6,x")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails reporting an unreadable line range
    assert_eq!(exit, 1, "{output}");
    assert!(output.contains("unreadable line range '4-6,x'"), "{output}");
    Ok(())
}
