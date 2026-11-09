//! BUG-506 — update-step text can carry the new keyword.
//!
//! Feature: spec/features/update-step-text-can-carry-the-new-keyword.feature

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BODY: &str = "Feature: X\n\n  Scenario: S\n    Given I log in\n    Then it works\n";

/// Update step "Given I log in" with `updates`; return (envelope, step line).
fn update(updates: Value) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("spec/features/x.feature");
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(&path, BODY)?;
    let mut args = updates;
    args["feature"] = json!("x");
    args["scenario"] = json!("S");
    args["currentStep"] = json!("Given I log in");
    let result = dispatch_command(DispatchRequest {
        command: "update-step".to_string(),
        args_json: args.to_string(),
        project_root: root.path().to_path_buf(),
    });
    let envelope: Value = if result.success {
        serde_json::from_str(&result.data)?
    } else {
        json!({"success": false, "error": format!("{result:?}")})
    };
    let after = std::fs::read_to_string(&path)?;
    let step = after.lines().nth(3).unwrap_or_default().trim().to_string();
    Ok((envelope, step))
}

#[test]
fn a_keyword_at_the_start_of_the_new_text_replaces_the_step_keyword() -> TestResult {
    // @step Given a scenario with the step "Given I log in"
    // @step When I update that step with the text "And I log in again"
    let (envelope, step) = update(json!({"text": "And I log in again"}))?;
    assert_eq!(envelope["success"], true, "{envelope}");
    // @step Then the step reads "And I log in again"
    assert_eq!(step, "And I log in again");
    Ok(())
}

#[test]
fn text_without_a_keyword_keeps_the_current_keyword() -> TestResult {
    // @step Given a scenario with the step "Given I log in"
    // @step When I update that step with the text "I log in again"
    let (envelope, step) = update(json!({"text": "I log in again"}))?;
    assert_eq!(envelope["success"], true, "{envelope}");
    // @step Then the step reads "Given I log in again"
    assert_eq!(step, "Given I log in again");
    Ok(())
}

#[test]
fn a_keyword_option_that_contradicts_the_keyword_in_the_text_is_refused() -> TestResult {
    // @step Given a scenario with the step "Given I log in"
    // @step When I update that step with the keyword "When" and the text "Then I log in again"
    let (envelope, step) = update(json!({"keyword": "When", "text": "Then I log in again"}))?;
    // @step Then update-step fails naming both keywords
    assert_eq!(envelope["success"], false, "{envelope}");
    let error = envelope["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("When") && error.contains("Then"),
        "{envelope}"
    );
    // @step And the step still reads "Given I log in"
    assert_eq!(step, "Given I log in");
    Ok(())
}
