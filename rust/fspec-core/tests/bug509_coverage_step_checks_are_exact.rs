//! BUG-509 — coverage step checks accept only exact step comments.
//!
//! Feature: spec/features/coverage-step-checks-accept-only-exact-step-comments.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they open the session list and pick the newest session\n    Then they see the board\n";

const EXACT_GIVEN: &str = "    // @step Given a user\n";
const EXACT_WHEN: &str =
    "    // @step When they open the session list and pick the newest session\n";
const EXACT_THEN: &str = "    // @step Then they see the board\n";

/// A test file `tests/login_test.rs` holding `body`, mapped over all its lines.
fn project(body: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, text: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)
    };
    let test = format!("fn log_in() {{\n{body}}}\n");
    write("spec/features/login.feature", FEATURE)?;
    write("tests/login_test.rs", &test)?;
    let lines = format!("1-{}", test.lines().count());
    write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": [
            {"file": "tests/login_test.rs", "lines": lines, "implMappings": []}
        ]}]})
        .to_string(),
    )?;
    Ok(root)
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn audit(root: &Path) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let result = dispatch(
        root,
        "audit",
        json!({"feature": ["login"], "area": ["coverage"]}),
    );
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let report: Value = serde_json::from_str(&result.data)?;
    let output = report["output"].as_str().unwrap_or_default().to_string();
    Ok((report["exitCode"].clone(), output))
}

fn assert_missing(exit: &Value, output: &str, step: &str) {
    assert_eq!(exit, &json!(1), "{output}");
    assert!(
        output.contains(&format!("no @step comment for '{step}'")),
        "{output}"
    );
}

#[test]
fn a_plain_comment_without_the_step_marker_does_not_cover_a_step() -> TestResult {
    // @step Given the mapped lines hold the comment "// Given a user" without the step marker
    let root = project(&format!("    // Given a user\n{EXACT_WHEN}{EXACT_THEN}"))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports the step "Given a user" as missing
    assert_missing(&exit, &output, "Given a user");
    Ok(())
}

#[test]
fn a_step_comment_with_extra_words_does_not_cover_a_step() -> TestResult {
    // @step Given the mapped lines hold a step comment for "When they open the session list and pick the newest session again" while the step is "When they open the session list and pick the newest session"
    let root = project(&format!(
        "{EXACT_GIVEN}    // @step When they open the session list and pick the newest session again\n{EXACT_THEN}"
    ))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports the step "When they open the session list and pick the newest session" as missing
    assert_missing(
        &exit,
        &output,
        "When they open the session list and pick the newest session",
    );
    Ok(())
}

#[test]
fn a_step_comment_that_differs_only_in_letter_case_does_not_cover_a_step() -> TestResult {
    // @step Given the mapped lines hold a step comment for "Then they see the Board" while the step is "Then they see the board"
    let root = project(&format!(
        "{EXACT_GIVEN}{EXACT_WHEN}    // @step Then they see the Board\n"
    ))?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports the step "Then they see the board" as missing
    assert_missing(&exit, &output, "Then they see the board");
    Ok(())
}

#[test]
fn an_exact_step_comment_after_any_documented_comment_marker_covers_the_step() -> TestResult {
    // @step Given the mapped lines hold exact step comments after the markers "//", "#" and "--"
    let root = project(
        "    // @step Given a user\n    # @step When they open the session list and pick the newest session\n    -- @step Then they see   the board\n",
    )?;
    // @step When the coverage is audited
    let (exit, output) = audit(root.path())?;
    // @step Then the audit reports all mappings valid
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("No problems found"), "{output}");
    Ok(())
}

#[test]
fn link_coverage_refuses_a_range_whose_step_comment_is_only_similar() -> TestResult {
    // @step Given a test whose step comment for "When they open the session list and pick the newest session" reads "When they open the session list and pick the newest session again"
    let root = project(&format!(
        "{EXACT_GIVEN}    // @step When they open the session list and pick the newest session again\n{EXACT_THEN}"
    ))?;
    // @step When that range is linked to the scenario
    let result = dispatch(
        root.path(),
        "link-coverage",
        json!({"featureName": "login", "scenario": "Log in",
            "testFile": "tests/login_test.rs", "testLines": "1-5", "replace": true}),
    );
    // @step Then link-coverage fails naming the step "When they open the session list and pick the newest session"
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(
        text.contains("✗ When they open the session list and pick the newest session"),
        "{text}"
    );
    Ok(())
}
