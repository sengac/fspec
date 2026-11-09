//! generate-coverage ends with a system reminder that coverage files are
//! created empty, that link-coverage populates them, the seven-step ACDD
//! coverage workflow and example link-coverage / show-coverage commands, in
//! a normal run and in a dry run.
//!
//! Feature: spec/features/add-system-reminder-to-generate-coverage-command.feature
//!
//! Code: rust/fspec-core/src/commands/generate_coverage/report.rs
//! (SYSTEM_REMINDER, appended by the regular and the dry-run renderers).

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A project with one feature file and no coverage file.
fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let features = tmp.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    std::fs::write(
        features.join("login.feature"),
        "Feature: User Login\n\n  Scenario: Login\n    Given I am on the login page\n    When I enter valid credentials\n    Then I see the dashboard\n",
    )?;
    Ok(tmp)
}

/// Run generate-coverage with `args`; its output.
fn generate(dir: &Path, args: Value) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "generate-coverage".to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("generate-coverage failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The one `<system-reminder>` block at the end of `output`.
fn reminder(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    let start = output
        .find("<system-reminder>\n")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let block = &output[start..];
    assert!(block.trim_end().ends_with("</system-reminder>"), "{output}");
    Ok(block)
}

fn has(block: &str, text: &str) {
    assert!(block.contains(text), "missing {text:?} in:\n{block}");
}

const WORKFLOW: &str = "ACDD Coverage Workflow:
  1. Write specifications (feature files)
  2. Generate coverage files: fspec generate-coverage
  3. Write tests: Write failing tests for scenarios
  4. Link tests: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --test-lines <range>
  5. Implement code: Write code AND wire up integration points
  6. Link implementation: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --impl-file <path> --impl-lines <lines>
  7. Verify coverage: fspec show-coverage <feature>
";

#[test]
fn display_reminder_after_generate_coverage_with_no_arguments() -> TestResult {
    // @step Given I have feature files in spec/features/ directory
    let tmp = project()?;

    // @step When I run `fspec generate-coverage`
    let output = generate(tmp.path(), json!({}))?;

    // @step Then the command should succeed
    assert!(
        tmp.path()
            .join("spec/features/login.feature.coverage")
            .exists(),
        "{output}"
    );

    // @step And the output should display a system-reminder
    let block = reminder(&output)?;

    // @step And the reminder should explain that coverage files are created empty
    has(block, "CRITICAL: Coverage files are created EMPTY");

    // @step And the reminder should mention that link-coverage must be used to populate them
    has(block, "must be manually POPULATES using link-coverage.");

    // @step And the reminder should include example link-coverage command for linking tests
    has(
        block,
        "  # Link test to scenario\n  fspec link-coverage user-authentication --scenario \"Login with valid credentials\" \\\n    --test-file src/__tests__/auth.test.ts --test-lines 45-62\n",
    );

    // @step And the reminder should include example link-coverage command for linking implementation
    has(
        block,
        "  # Link implementation to test mapping\n  fspec link-coverage user-authentication --scenario \"Login with valid credentials\" \\\n    --test-file src/__tests__/auth.test.ts \\\n    --impl-file src/auth/login.ts --impl-lines 10-24\n",
    );

    // @step And the reminder should mention show-coverage as a verification step
    has(
        block,
        "  # Verify coverage status\n  fspec show-coverage user-authentication\n",
    );
    Ok(())
}

#[test]
fn display_reminder_after_generate_coverage_dry_run() -> TestResult {
    // @step Given I have feature files in spec/features/ directory
    let tmp = project()?;

    // @step When I run `fspec generate-coverage --dry-run`
    let output = generate(tmp.path(), json!({"dryRun": true}))?;

    // @step Then the command should succeed
    assert!(
        !tmp.path()
            .join("spec/features/login.feature.coverage")
            .exists(),
        "a dry run writes nothing: {output}"
    );

    // @step And the output should display a system-reminder
    let block = reminder(&output)?;

    // @step And the reminder should explain the seven-step ACDD coverage workflow
    has(block, WORKFLOW);

    // @step And the reminder should reference link-coverage command
    has(block, "link-coverage POPULATES coverage files");
    Ok(())
}

#[test]
fn reminder_explains_difference_between_generate_coverage_and_link_coverage() -> TestResult {
    // @step Given I have feature files in spec/features/ directory
    let tmp = project()?;

    // @step When I run `fspec generate-coverage`
    let output = generate(tmp.path(), json!({}))?;
    let block = reminder(&output)?;

    // @step Then the system-reminder should state "generate-coverage creates EMPTY coverage files"
    has(
        block,
        "  • generate-coverage creates EMPTY coverage files\n",
    );

    // @step And the system-reminder should state "link-coverage POPULATES coverage files with test and implementation mappings"
    has(
        block,
        "  • link-coverage POPULATES coverage files with test and implementation mappings\n",
    );

    // @step And the reminder should emphasize that generation and linking are separate steps
    has(
        block,
        "Understanding generate-coverage vs link-coverage (separate steps):\n",
    );
    Ok(())
}

#[test]
fn reminder_shows_complete_acdd_workflow_with_coverage_commands() -> TestResult {
    // @step Given I have feature files in spec/features/ directory
    let tmp = project()?;

    // @step When I run `fspec generate-coverage`
    let output = generate(tmp.path(), json!({}))?;
    let block = reminder(&output)?;

    // @step Then the system-reminder should list the seven ACDD Coverage Workflow steps from writing specifications to fspec show-coverage <feature>
    has(block, WORKFLOW);

    // @step And the link steps should give the link-coverage commands for tests and for implementation
    has(
        block,
        "  4. Link tests: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --test-lines <range>\n",
    );
    has(
        block,
        "  6. Link implementation: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --impl-file <path> --impl-lines <lines>\n",
    );
    Ok(())
}
