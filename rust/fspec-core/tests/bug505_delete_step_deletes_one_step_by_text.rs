//! BUG-505 — delete-step deletes one step by its text.
//!
//! Feature: spec/features/delete-step-deletes-one-step-by-its-text.feature

use codelet_fspec_core::help::{configs, format_command_help};
use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TWO_USERS: &str =
    "Feature: X\n\n  Scenario: S\n    Given a user\n    And a user\n    Then it works\n";

/// Write `body` as spec/features/x.feature and delete `step` from scenario S.
fn delete(body: &str, step: &str) -> Result<(DispatchResult, String), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("spec/features/x.feature");
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(&path, body)?;
    let result = dispatch_command(DispatchRequest {
        command: "delete-step".to_string(),
        args_json: json!({"feature": "x", "scenario": "S", "step": step}).to_string(),
        project_root: root.path().to_path_buf(),
    });
    Ok((result, std::fs::read_to_string(&path)?))
}

#[test]
fn text_that_matches_two_steps_is_refused() -> TestResult {
    // @step Given a scenario with the steps "Given a user" and "And a user"
    let body = TWO_USERS;
    // @step When I delete the step "a user"
    let (result, after) = delete(body, "a user")?;
    // @step Then delete-step fails naming both matching lines
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(text.contains("line 4") && text.contains("line 5"), "{text}");
    // @step And both steps are still in the scenario
    assert_eq!(after, body);
    Ok(())
}

#[test]
fn keyword_qualified_text_deletes_the_one_step_it_names() -> TestResult {
    // @step Given a scenario with the steps "Given a user" and "And a user"
    let body = TWO_USERS;
    // @step When I delete the step "And a user"
    let (result, after) = delete(body, "And a user")?;
    assert!(result.success, "{result:?}");
    // @step Then only "Given a user" is left
    assert_eq!(
        after,
        "Feature: X\n\n  Scenario: S\n    Given a user\n    Then it works\n"
    );
    Ok(())
}

#[test]
fn deleting_a_step_deletes_its_data_table() -> TestResult {
    // @step Given a scenario whose step "Given rows" has a data table
    let body = "Feature: X\n\n  Scenario: S\n    Given rows\n      | a | b |\n      | 1 | 2 |\n    Then it works\n";
    // @step When I delete the step "Given rows"
    let (result, after) = delete(body, "Given rows")?;
    assert!(result.success, "{result:?}");
    // @step Then no row of that table is left in the file
    assert_eq!(after, "Feature: X\n\n  Scenario: S\n    Then it works\n");
    Ok(())
}

#[test]
fn the_help_describes_the_step_text_argument() {
    // @step When I read the delete-step help
    let help = format_command_help(&configs::delete_step::CONFIG);
    // @step Then it describes the step argument as the step text with or without its keyword
    assert!(help.contains("<step>"), "{help}");
    assert!(help.contains("with or without its keyword"), "{help}");
    // @step And it mentions neither a step index nor --force
    assert!(!help.contains("index"), "{help}");
    assert!(!help.contains("--force"), "{help}");
}
