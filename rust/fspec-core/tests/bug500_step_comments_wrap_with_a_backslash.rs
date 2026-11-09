//! BUG-500 — step comments wrap with a trailing backslash.
//!
//! Feature: spec/features/step-comments-wrap-with-a-trailing-backslash.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Sessions\n\n  Scenario: List\n    Given the session list shows every saved session with its title and age\n";

fn project(test_file: &str, test: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/sessions.feature", FEATURE)?;
    write(test_file, test)?;
    let lines = format!("1-{}", test.lines().count());
    write(
        "spec/features/sessions.feature.coverage",
        &json!({"scenarios": [{"name": "List", "testMappings": [
            {"file": test_file, "lines": lines, "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    Ok(root)
}

fn audit(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: json!({"feature": ["sessions"], "area": ["coverage"]}).to_string(),
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
fn a_step_comment_wrapped_with_a_trailing_backslash_matches_the_step() -> TestResult {
    // @step Given a test whose @step comment for a long step ends with a backslash and continues on the next comment line
    let root = project(
        "tests/sessions_test.rs",
        "fn list() {\n    // @step Given the session list \\\n    //     shows every saved session with its title and age\n}\n",
    )?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn a_hash_comment_can_continue_twice() -> TestResult {
    // @step Given a Python test whose @step comment continues over three comment lines with backslashes
    let root = project(
        "tests/test_sessions.py",
        "def test_list():\n    # @step Given the session list \\\n    #   shows every saved session \\\n    #   with its title and age\n    pass\n",
    )?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn a_comment_line_after_a_step_comment_without_a_backslash_is_not_joined() -> TestResult {
    // @step Given a test whose @step comment holds only the start of a long step and the rest is on the next comment line without a backslash
    let root = project(
        "tests/sessions_test.rs",
        "fn list() {\n    // @step Given the session list\n    // shows every saved session with its title and age\n}\n",
    )?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit fails naming the long step
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.contains("Given the session list shows every saved session with its title and age"),
        "{output}"
    );
    Ok(())
}
