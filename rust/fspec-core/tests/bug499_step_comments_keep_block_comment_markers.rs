//! BUG-499 — step comments keep text after a block-comment marker.
//!
//! Feature: spec/features/step-comments-keep-text-after-a-block-comment-marker.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn project(feature: &str, test: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/lint.feature", feature)?;
    write("tests/lint_test.ts", test)?;
    let lines = format!("1-{}", test.lines().count());
    write(
        "spec/features/lint.feature.coverage",
        &json!({"scenarios": [{"name": "Lint", "testMappings": [
            {"file": "tests/lint_test.ts", "lines": lines, "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    Ok(root)
}

fn audit(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["lint"], "area": ["coverage"]}).to_string(),
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
fn a_line_comment_whose_step_names_a_glob_matches_the_step() -> TestResult {
    // @step Given a scenario step that names a glob pattern containing a block-comment terminator
    let feature = "Feature: Lint\n\n  Scenario: Lint\n    Given \"**/*.ts\" names the files to lint\n    Then every match under src is linted\n";
    // @step And a test whose line @step comment repeats that step exactly
    let test = "test('lint', () => {\n  // @step Given \"**/*.ts\" names the files to lint\n  // @step Then every match under src is linted\n});\n";
    let root = project(feature, test)?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn a_one_line_block_comment_still_matches_without_its_terminator() -> TestResult {
    // @step Given a test whose @step comment is a one-line block comment for the step "Given a user"
    let feature = "Feature: Lint\n\n  Scenario: Lint\n    Given a user\n";
    let root = project(
        feature,
        "test('lint', () => {\n  /* @step Given a user */\n});\n",
    )?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}
