//! BUG-525 — fspec audit checks test mapping ranges against the file length.
//!
//! Feature: spec/features/audit-coverage-checks-test-mapping-ranges-against-the-file-length.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

/// A 19-line test file whose @step comments are on lines 10 to 12.
fn test_file() -> String {
    let mut lines: Vec<String> = (1..=9).map(|n| format!("// filler {n}")).collect();
    lines.push("    // @step Given a user".to_string());
    lines.push("    // @step When they log in".to_string());
    lines.push("    // @step Then they see the board".to_string());
    lines.extend((13..=19).map(|n| format!("// filler {n}")));
    format!("{}\n", lines.join("\n"))
}

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
    write("tests/login_test.rs", &test_file())?;
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
fn a_test_range_past_the_end_of_the_file_is_reported() -> TestResult {
    // @step Given a test file of 19 lines whose @step comments are on lines 10 to 12
    // @step And scenario "Log in" maps that file's lines 10-20
    let root = project("10-20")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails saying the mapping lies outside the file of 19 lines
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("tests/login_test.rs:10-20: lies outside the file (19 lines)"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_comma_list_part_past_the_end_of_the_file_is_reported() -> TestResult {
    // @step Given a test file of 19 lines whose @step comments are on lines 10 to 12
    // @step And scenario "Log in" maps that file's lines "10-12,18-25"
    let root = project("10-12,18-25")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails saying the mapping lies outside the file of 19 lines
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("tests/login_test.rs:10-12,18-25: lies outside the file (19 lines)"),
        "{output}"
    );
    Ok(())
}

#[test]
fn a_test_range_inside_the_file_passes() -> TestResult {
    // @step Given a test file of 19 lines whose @step comments are on lines 10 to 12
    // @step And scenario "Log in" maps that file's lines 10-19
    let root = project("10-19")?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}
