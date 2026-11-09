//! BUG-486 — delete-scenario removes the whole scenario block.
//!
//! Feature: spec/features/delete-scenario-removes-the-whole-scenario-block.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Write `body` as spec/features/x.feature, delete `scenario`, return the file.
fn delete(body: &str, scenario: &str) -> Result<String, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("spec/features/x.feature");
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(&path, body)?;
    let result = run(root.path(), scenario);
    if !result.success {
        return Err(format!("delete-scenario failed: {result:?}").into());
    }
    Ok(std::fs::read_to_string(&path)?)
}

fn run(root: &Path, scenario: &str) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: "delete-scenario".to_string(),
        args_json: json!({"feature": "x", "scenario": scenario}).to_string(),
        project_root: root.to_path_buf(),
    })
}

#[test]
fn tags_and_comments_above_the_deleted_scenario_go_with_it() -> TestResult {
    // @step Given a feature where scenario "Two" has a tag line and a comment line above it and scenario "Three" has its own tag
    let body = "Feature: X\n\n  @first\n  Scenario: One\n    Given a\n\n  @second\n  # comment above two\n  Scenario: Two\n    Given c\n    # trailing note for two\n\n  @third\n  Scenario: Three\n    Given e\n";
    // @step When I delete scenario "Two"
    let after = delete(body, "Two")?;
    // @step Then the tag and comment lines of "Two" are gone
    assert!(!after.contains("@second"), "{after}");
    assert!(!after.contains("comment above two"), "{after}");
    assert!(!after.contains("trailing note for two"), "{after}");
    // @step And scenario "Three" still has exactly its own tag
    assert!(
        after.ends_with("    Given a\n\n  @third\n  Scenario: Three\n    Given e\n"),
        "{after}"
    );
    Ok(())
}

#[test]
fn the_data_table_of_the_last_step_goes_with_the_scenario() -> TestResult {
    // @step Given a feature where the last step of scenario "Two" has a data table
    let body = "Feature: X\n\n  Scenario: One\n    Given a\n\n  Scenario: Two\n    Given rows\n      | a | b |\n      | 1 | 2 |\n\n  Scenario: Three\n    Given e\n";
    // @step When I delete scenario "Two"
    let after = delete(body, "Two")?;
    // @step Then no row of that table is left in the file
    assert_eq!(
        after,
        "Feature: X\n\n  Scenario: One\n    Given a\n\n  Scenario: Three\n    Given e\n"
    );
    Ok(())
}

#[test]
fn the_examples_of_a_deleted_outline_go_with_it() -> TestResult {
    // @step Given a feature with a Scenario Outline "Three" followed by scenario "Four"
    let body = "Feature: X\n\n  Scenario: One\n    Given a\n\n  Scenario Outline: Three\n    Given <n>\n\n    Examples:\n      | n |\n      | 1 |\n\n  Scenario: Four\n    Given z\n";
    // @step When I delete scenario "Three"
    let after = delete(body, "Three")?;
    // @step Then the Examples table is gone
    assert!(!after.contains("Examples:"), "{after}");
    assert!(!after.contains("| n |"), "{after}");
    // @step And scenario "Four" is unchanged
    assert!(
        after.ends_with("    Given a\n\n  Scenario: Four\n    Given z\n"),
        "{after}"
    );
    Ok(())
}

#[test]
fn deleting_the_last_scenario_leaves_no_dangling_lines() -> TestResult {
    // @step Given a feature whose last scenario has tags interleaved with comments above it and a trailing comment
    let body = "Feature: X\n\n  Scenario: One\n    Given a\n    Then b\n\n  @second\n  # comment above\n  @third\n  Scenario: Last\n    Given e\n    # trailing\n";
    // @step When I delete the last scenario
    let after = delete(body, "Last")?;
    // @step Then the file ends with the previous scenario's last step followed by one newline
    assert_eq!(
        after,
        "Feature: X\n\n  Scenario: One\n    Given a\n    Then b\n"
    );
    Ok(())
}

#[test]
fn deleting_scenarios_by_tag_also_removes_comments_between_their_tags() -> TestResult {
    // @step Given a feature where scenario "Two" has a comment line between its tag lines
    let root = tempfile::tempdir()?;
    let path = root.path().join("spec/features/x.feature");
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(
        &path,
        "Feature: X\n\n  Scenario: One\n    Given a\n\n  @gone\n  # why it goes\n  @slow\n  Scenario: Two\n    Given c\n\n  Scenario: Three\n    Given e\n",
    )?;
    // @step When I delete the scenarios tagged with the tag of "Two"
    let result = dispatch_command(DispatchRequest {
        command: "delete-scenarios".to_string(),
        args_json: json!({"tags": ["@slow"]}).to_string(),
        project_root: root.path().to_path_buf(),
    });
    assert!(result.success, "{result:?}");
    // @step Then neither the comment nor the tag lines of "Two" are left in the file
    let after = std::fs::read_to_string(&path)?;
    assert!(!after.contains("@gone"), "{after}");
    assert!(!after.contains("why it goes"), "{after}");
    assert!(!after.contains("Scenario: Two"), "{after}");
    Ok(())
}
