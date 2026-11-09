//! FOUND-007 — project-management commands refuse to run without
//! spec/foundation.json, tell the AI to run discover-foundation and to retry
//! the original command afterwards; read-only commands such as validate are
//! exempt.
//!
//! Feature: spec/features/foundation-existence-check-in-commands.feature
//!
//! Code: rust/fspec-core/src/io/ensure.rs (check_foundation_exists,
//! build_foundation_missing_error), called by commands/board.rs and
//! commands/create_story.rs; commands/validate.rs has no check.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FOUNDATION_MISSING: &str = "Error: Project foundation not found.";

fn dispatch(dir: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    })
}

/// The error text of a failed dispatch.
fn error_of(result: &DispatchResult) -> String {
    assert!(!result.success, "expected the command to fail: {result:?}");
    result.error.clone().unwrap_or_default()
}

/// A project directory with an empty spec/ and no spec/foundation.json.
fn project_without_foundation() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    std::fs::create_dir_all(tmp.path().join("spec"))?;
    assert!(!tmp.path().join("spec/foundation.json").exists());
    Ok(tmp)
}

#[test]
fn run_board_command_without_foundation_json() -> TestResult {
    // @step Given I am in a project directory
    // @step And the file "spec/foundation.json" does not exist
    let tmp = project_without_foundation()?;

    // @step When I run 'fspec board'
    let result = dispatch(tmp.path(), "board", json!({"format": "json"}));

    // @step Then the command should fail
    let error = error_of(&result);

    // @step And the output should display an error message
    assert!(error.starts_with(FOUNDATION_MISSING), "{error}");

    // @step And the error message should instruct me to run 'fspec discover-foundation'
    assert!(
        error.contains("  Step 1: fspec discover-foundation (creates draft)"),
        "{error}"
    );
    assert!(
        error.contains("  1. Run: fspec discover-foundation"),
        "{error}"
    );

    // @step And a system reminder should tell me to retry 'fspec board' after discover-foundation completes
    assert!(error.contains("<system-reminder>"), "{error}");
    assert!(
        error.contains("The command 'fspec board' requires a project foundation document."),
        "{error}"
    );
    assert!(
        error.contains("  3. After foundation.json is created, retry: fspec board\n"),
        "{error}"
    );
    assert!(
        error.contains("DO NOT forget to run: fspec board\n"),
        "{error}"
    );
    assert!(!tmp.path().join("spec/foundation.json").exists());
    Ok(())
}

#[test]
fn run_create_story_without_foundation_json() -> TestResult {
    // @step Given I am in a project directory
    // @step And the file "spec/foundation.json" does not exist
    let tmp = project_without_foundation()?;

    // @step When I run 'fspec create-story AUTH "Login"'
    let result = dispatch(
        tmp.path(),
        "create-story",
        json!({"prefix": "AUTH", "title": "Login"}),
    );

    // @step Then the command should fail
    let error = error_of(&result);

    // @step And the output should display an error message
    assert!(error.starts_with(FOUNDATION_MISSING), "{error}");

    // @step And the error message should instruct me to run 'fspec discover-foundation'
    assert!(
        error.contains("  Step 1: fspec discover-foundation (creates draft)"),
        "{error}"
    );

    // @step And a system reminder should include the original command 'fspec create-story AUTH "Login"' to retry
    assert!(
        error.contains(
            "  3. After foundation.json is created, retry: fspec create-story AUTH \"Login\"\n"
        ),
        "{error}"
    );
    assert!(
        error.contains("DO NOT forget to run: fspec create-story AUTH \"Login\"\n"),
        "{error}"
    );
    assert!(
        !tmp.path().join("spec/work-units.json").exists()
            || !std::fs::read_to_string(tmp.path().join("spec/work-units.json"))?
                .contains("AUTH-001"),
        "no story may be created"
    );
    Ok(())
}

#[test]
fn run_board_command_with_foundation_json_present() -> TestResult {
    // @step Given I am in a project directory
    // @step And the file "spec/foundation.json" exists
    let tmp = project_without_foundation()?;
    std::fs::write(tmp.path().join("spec/foundation.json"), "{}\n")?;

    // @step When I run 'fspec board'
    let result = dispatch(tmp.path(), "board", json!({"format": "json"}));

    // @step Then the command should execute normally
    assert!(result.success, "board failed: {result:?}");
    let board: Value = serde_json::from_str(&result.data)?;
    assert!(board.is_object(), "{board}");

    // @step And no foundation check error should be displayed
    assert!(!result.data.contains(FOUNDATION_MISSING), "{}", result.data);
    assert!(
        !result.data.contains("FOUNDATION MISSING"),
        "{}",
        result.data
    );
    Ok(())
}

#[test]
fn run_validate_command_without_foundation_json_read_only_exempt() -> TestResult {
    // @step Given I am in a project directory
    // @step And the file "spec/foundation.json" does not exist
    let tmp = project_without_foundation()?;
    std::fs::create_dir_all(tmp.path().join("spec/features"))?;
    std::fs::write(
        tmp.path().join("spec/features/login.feature"),
        "Feature: Login\n\n  Scenario: Valid login\n    Given a user\n    When they log in\n    Then they see the dashboard\n",
    )?;

    // @step When I run 'fspec validate'
    let result = dispatch(tmp.path(), "validate", json!({}));

    // @step Then the command should execute normally
    assert!(result.success, "validate failed: {result:?}");

    // @step And no foundation check error should be displayed
    assert!(!result.data.contains(FOUNDATION_MISSING), "{}", result.data);
    assert!(
        !result.data.contains("FOUNDATION MISSING"),
        "{}",
        result.data
    );

    // @step And validation should proceed for feature files
    let data: Value = serde_json::from_str(&result.data)?;
    let output = data["output"].as_str().unwrap_or_default();
    assert!(
        output.contains("✓ spec/features/login.feature is valid"),
        "{output}"
    );
    assert!(!tmp.path().join("spec/foundation.json").exists());
    Ok(())
}
