//! REMIND-014 — the move from specifying to testing requires AST research,
//! and a story that passes the objective checks gets the architectural
//! review reminder.
//!
//! Feature: spec/features/enhance-fspec-review-with-architecture-alignment-and-ast-verification.feature
//!
//! `perform_review_validation` (update_work_unit_status/review.rs) blocks a
//! story without Example Mapping, without architectural notes, or without
//! an attachment named `ast-research*`. A story that passes gets the
//! subjective review reminder, which lists the attachments and notes.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NOTE: &str = "Refactoring: Extract validation logic to shared utility";
const ATTACHMENT: &str = "spec/attachments/WORK-001/ast-research-validate-feature.md";

/// A story WORK-001 in specifying with Example Mapping, one architectural
/// note, the given attachments and a scenario-bearing feature tagged
/// @WORK-001 that holds no prefill.
fn project(dir: &Path, attachments: &[&str]) -> TestResult {
    std::fs::create_dir_all(dir.join("spec/features"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-01-01T00:00:00.000Z"},
            "workUnits": {"WORK-001": {
                "id": "WORK-001", "title": "Validate features", "type": "story", "status": "specifying",
                "rules": [{"id": 0, "text": "Features are validated once", "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}],
                "examples": [{"id": 0, "text": "A valid feature passes", "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}],
                "architectureNotes": [{"id": 0, "text": NOTE, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}],
                "attachments": attachments,
                "estimate": 2,
                "stateHistory": [
                    {"state": "backlog", "timestamp": "2026-01-01T00:00:00.000Z"},
                    {"state": "specifying", "timestamp": "2026-01-01T00:00:00.000Z"}
                ],
                "createdAt": "2026-01-01T00:00:00.000Z", "updatedAt": "2026-01-01T00:00:00.000Z"
            }},
            "states": {"backlog": [], "specifying": ["WORK-001"], "testing": [], "implementing": [],
                       "validating": [], "done": [], "blocked": []},
        }))?,
    )?;
    std::fs::write(
        dir.join("spec/features/validate-features.feature"),
        "@WORK-001\nFeature: Validate features\n\n  Background: User Story\n    As a developer\n    I want to validate features\n    So that broken specs are caught\n\n  Scenario: A valid feature passes\n    Given a valid feature file\n    When I validate it\n    Then it passes\n",
    )?;
    for attachment in attachments {
        let path = dir.join(attachment);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            "# AST research\n\nvalidateFeature: src/validate.ts:12\n",
        )?;
    }
    Ok(())
}

fn move_to_testing(dir: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "WORK-001", "status": "testing"}).to_string(),
        project_root: dir.to_path_buf(),
    })
}

fn saved_unit(dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let board: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("spec/work-units.json"))?)?;
    Ok(board["workUnits"]["WORK-001"].clone())
}

/// The content of the only `<system-reminder>` block in `output`.
fn single_block(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    let start = output
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let end = output
        .find("</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{output}"))?;
    Ok(&output[start..end])
}

#[test]
fn block_transition_when_ast_research_is_missing() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit in specifying state with completed feature file
    project(dir, &[])?;

    // @step When I run 'fspec update-work-unit-status WORK-001 testing'
    let result = move_to_testing(dir);

    // @step Then the command should fail with error code 1
    assert!(!result.success, "{result:?}");

    // @step And the work unit has no AST research attachments
    assert_eq!(saved_unit(dir)?["attachments"], json!([]));

    // @step And the error message should contain 'Cannot transition to testing - no AST research performed during discovery'
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("Cannot transition to testing - no AST research performed during discovery"),
        "{error}"
    );

    // @step And the work unit status should remain 'specifying'
    let unit = saved_unit(dir)?;
    assert_eq!(unit["status"], "specifying");
    Ok(())
}

#[test]
fn pass_review_with_proper_architecture_documentation_and_ast_research() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit with architectural note 'Refactoring: Extract validation logic to shared utility'
    project(dir, &[ATTACHMENT])?;
    assert_eq!(saved_unit(dir)?["architectureNotes"][0]["text"], NOTE);

    // @step When I run 'fspec update-work-unit-status WORK-001 testing'
    let result = move_to_testing(dir);

    // @step Then the command should succeed with status code 0
    assert!(result.success, "{result:?}");
    let unit = saved_unit(dir)?;

    // @step And the work unit has an AST research attachment
    assert_eq!(unit["attachments"], json!([ATTACHMENT]));

    // @step And the work unit has a completed feature file with no prefill placeholders
    let feature = std::fs::read_to_string(dir.join("spec/features/validate-features.feature"))?;
    assert!(!feature.contains('['), "{feature}");

    // @step And the work unit status should be 'testing'
    assert_eq!(unit["status"], "testing");

    // @step And the review validation should pass all objective ACDD checks
    assert!(
        result
            .data
            .contains("Work unit WORK-001 passed objective ACDD checks but requires AI analysis:"),
        "{}",
        result.data
    );
    Ok(())
}

#[test]
fn architectural_review_reminder_lists_the_ast_research_attachments_for_the_reinvention_check(
) -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a work unit with AST research attachment 'spec/attachments/WORK-001/ast-research-validate-feature.md' and architectural notes
    project(dir, &[ATTACHMENT])?;

    // @step When I run 'fspec update-work-unit-status WORK-001 testing'
    let result = move_to_testing(dir);

    // @step Then the command should succeed with status code 0
    assert!(result.success, "{result:?}");
    let block = single_block(&result.data)?;

    // @step And a system-reminder should list the AST research attachment and the architectural notes
    assert!(
        block.contains(&format!("AST RESEARCH ATTACHMENTS:\n  1. {ATTACHMENT}\n")),
        "{block}"
    );
    assert!(
        block.contains(&format!("ARCHITECTURAL NOTES:\n  1. {NOTE}\n")),
        "{block}"
    );

    // @step And the system-reminder should tell the AI to check for duplicate function names
    assert!(
        block.contains(
            "3. Verify not reinventing existing utilities (check for duplicate function names)"
        ),
        "{block}"
    );

    // @step And the system-reminder should tell the AI to revert to specifying if issues are found
    assert!(
        block.contains(
            "If issues found: Revert to specifying: fspec update-work-unit-status WORK-001 specifying"
        ),
        "{block}"
    );
    Ok(())
}
