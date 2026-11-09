//! BUG-044 — AI agents skip docstring step validation by using --skip-step-validation flag.
//!
//! Feature: spec/features/ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag.feature
//!
//! link-coverage reads the work unit type from the feature file's first
//! `@PREFIX-NNN` tag and spec/work-units.json: `--skip-step-validation` is
//! refused for story and bug work units and allowed, with a warning, for
//! tasks; the missing-step system-reminder offers the skip flag to tasks only.

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A temporary project holding one feature, its coverage file, one test
/// file and the work unit the feature is tagged with.
struct Project(tempfile::TempDir);

impl Project {
    /// Feature `name` tagged `@id` (a work unit of type `kind`) with one
    /// scenario `scenario` holding `steps`, and test file `test_file` of
    /// `test_lines` lines without any @step comment.
    fn new(
        id: &str,
        kind: &str,
        name: &str,
        scenario: &str,
        steps: &[&str],
        test_file: &str,
        test_lines: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        let body: Vec<String> = steps.iter().map(|step| format!("    {step}")).collect();
        project.write(
            &format!("spec/features/{name}.feature"),
            &format!(
                "@{id}\nFeature: {name}\n\n  Scenario: {scenario}\n{}\n",
                body.join("\n")
            ),
        )?;
        project.write(
            &format!("spec/features/{name}.feature.coverage"),
            &json!({"scenarios": [{"name": scenario, "testMappings": []}]}).to_string(),
        )?;
        project.write(
            "spec/work-units.json",
            &json!({"workUnits": {id: {"id": id, "type": kind}}}).to_string(),
        )?;
        let code: Vec<String> = (1..=test_lines)
            .map(|n| format!("  expect(step{n}).toBe(true);"))
            .collect();
        project.write(test_file, &format!("{}\n", code.join("\n")))?;
        Ok(project)
    }

    fn write(&self, rel: &str, body: &str) -> std::io::Result<()> {
        let path = self.0.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }

    fn read(&self, rel: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.0.path().join(rel))
    }

    fn link_coverage(&self, args: Value) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: "link-coverage".to_string(),
            args_json: args.to_string(),
            project_root: self.0.path().to_path_buf(),
        })
    }
}

/// The error of a failed dispatch.
fn failure(result: &DispatchResult) -> Result<String, Box<dyn std::error::Error>> {
    if result.success {
        return Err(format!("link-coverage succeeded: {result:?}").into());
    }
    Ok(result.error.clone().unwrap_or_default())
}

/// The line the task-only missing-step reminder uses to offer the skip flag.
const SKIP_SUGGESTION: &str =
    "fspec link-coverage <feature> --scenario <name> ... --skip-step-validation";

#[test]
fn attempt_to_skip_step_validation_for_story_work_unit_fails_with_strict_error() -> TestResult {
    // @step Given I have a story work unit "AUTH-001" with a feature file
    // @step And the feature file has a scenario "Login with valid credentials"
    // @step And I have a test file "src/__tests__/auth.test.ts" with test code but missing step comments
    let project = Project::new(
        "AUTH-001",
        "story",
        "user-login",
        "Login with valid credentials",
        &[
            "Given I am on the login page",
            "When I enter valid credentials",
            "Then I see the dashboard",
        ],
        "src/__tests__/auth.test.ts",
        25,
    )?;
    let coverage_before = project.read("spec/features/user-login.feature.coverage")?;

    // @step When I run "fspec link-coverage user-login --scenario 'Login with valid credentials' --test-file src/__tests__/auth.test.ts --test-lines 10-20 --skip-step-validation"
    let result = project.link_coverage(json!({
        "featureName": "user-login",
        "scenario": "Login with valid credentials",
        "testFile": "src/__tests__/auth.test.ts",
        "testLines": "10-20",
        "skipStepValidation": true
    }));

    // @step Then the command should fail with exit code 1
    let error = failure(&result)?;
    assert_eq!(
        project.read("spec/features/user-login.feature.coverage")?,
        coverage_before
    );

    // @step And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    assert!(
        error.contains("skip-step-validation flag is ONLY allowed for task work units"),
        "{error}"
    );

    // @step And the error message should contain "Story and bug work units require MANDATORY step validation"
    assert!(
        error.contains("Story and bug work units require MANDATORY step validation"),
        "{error}"
    );

    // @step And the error message should warn "Attempting to skip step validation will be detected and require going back to fix docstrings"
    assert!(
        error.contains(
            "Attempting to skip step validation will be detected and require going back to fix docstrings"
        ),
        "{error}"
    );

    // @step And the error message should NOT suggest using --skip-step-validation flag
    assert!(!error.contains(SKIP_SUGGESTION), "{error}");
    assert!(!error.contains("To override validation"), "{error}");
    assert!(
        error.contains("Remove the --skip-step-validation flag from your command"),
        "{error}"
    );
    Ok(())
}

#[test]
fn attempt_to_skip_step_validation_for_bug_work_unit_fails_with_strict_error() -> TestResult {
    // @step Given I have a bug work unit "BUG-044" with a feature file
    // @step And the feature file has a scenario "Fix validation bypass"
    // @step And I have a test file "src/__tests__/validation.test.ts" with test code but missing step comments
    let feature = "ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag";
    let project = Project::new(
        "BUG-044",
        "bug",
        feature,
        "Fix validation bypass",
        &[
            "Given an agent links coverage",
            "When it passes the skip flag",
            "Then the link is refused",
        ],
        "src/__tests__/validation.test.ts",
        20,
    )?;

    // @step When I run "fspec link-coverage ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag --scenario 'Fix validation bypass' --test-file src/__tests__/validation.test.ts --test-lines 5-15 --skip-step-validation"
    let result = project.link_coverage(json!({
        "featureName": feature,
        "scenario": "Fix validation bypass",
        "testFile": "src/__tests__/validation.test.ts",
        "testLines": "5-15",
        "skipStepValidation": true
    }));

    // @step Then the command should fail with exit code 1
    let error = failure(&result)?;
    assert!(
        error.contains("This work unit is a bug work unit, detected from feature file tags."),
        "{error}"
    );

    // @step And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    assert!(
        error.contains("skip-step-validation flag is ONLY allowed for task work units"),
        "{error}"
    );

    // @step And the error message should contain "Bug work units require MANDATORY step validation"
    assert!(
        error.contains("Bug work units require MANDATORY step validation"),
        "{error}"
    );
    Ok(())
}

#[test]
fn skip_step_validation_for_task_work_unit_succeeds() -> TestResult {
    // @step Given I have a task work unit "TASK-001" with a feature file
    // @step And the feature file has a scenario "Setup infrastructure"
    // @step And I have a test file "src/__tests__/setup.test.ts" with test code but missing step comments
    let project = Project::new(
        "TASK-001",
        "task",
        "infrastructure-setup",
        "Setup infrastructure",
        &[
            "Given a fresh machine",
            "When the setup runs",
            "Then the services start",
        ],
        "src/__tests__/setup.test.ts",
        20,
    )?;

    // @step When I run "fspec link-coverage infrastructure-setup --scenario 'Setup infrastructure' --test-file src/__tests__/setup.test.ts --test-lines 5-15 --skip-step-validation"
    let result = project.link_coverage(json!({
        "featureName": "infrastructure-setup",
        "scenario": "Setup infrastructure",
        "testFile": "src/__tests__/setup.test.ts",
        "testLines": "5-15",
        "skipStepValidation": true
    }));

    // @step Then the command should succeed with exit code 0
    assert!(result.success, "{result:?}");
    let envelope: Value = serde_json::from_str(&result.data)?;
    let coverage: Value = serde_json::from_str(
        &project.read("spec/features/infrastructure-setup.feature.coverage")?,
    )?;
    assert_eq!(coverage["scenarios"][0]["testMappings"][0]["lines"], "5-15");

    // @step And the output should confirm "✓ Linked test mapping: src/__tests__/setup.test.ts:5-15"
    let message = envelope["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("✓ Linked test mapping: src/__tests__/setup.test.ts:5-15"),
        "{message}"
    );

    // @step And the output should contain a warning "⚠️  Step validation skipped (task work unit)"
    let warnings = envelope["warnings"].as_str().unwrap_or_default();
    assert!(
        warnings.contains("⚠️  Step validation skipped (task work unit)"),
        "{envelope}"
    );
    Ok(())
}

/// The missing-step reminder of a story or bug work unit: every step with
/// its exact `// @step` line, the MANDATORY line for `kind`, and no skip
/// option.
fn assert_strict_missing_step_reminder(
    error: &str,
    steps: &[&str],
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !error.contains("<system-reminder>") || !error.contains("Missing step comments:") {
        return Err(format!("no missing-step system-reminder: {error}").into());
    }
    for step in steps {
        if !error.contains(&format!("Add to test file: // @step {step}")) {
            return Err(format!("step '{step}' is not offered: {error}").into());
        }
    }
    if error.contains("--skip-step-validation") {
        return Err(format!("the skip flag is mentioned: {error}").into());
    }
    if !error.contains(&format!(
        "Step validation is MANDATORY for {kind} work units"
    )) {
        return Err(format!("no MANDATORY line for {kind}: {error}").into());
    }
    Ok(())
}

const RESET_STEPS: [&str; 3] = [
    "Given I am on the password reset page",
    "When I enter my email address",
    "Then I should receive a password reset email",
];

#[test]
fn story_work_unit_with_missing_step_comments_receives_strict_system_reminder_without_skip_option(
) -> TestResult {
    // @step Given I have a story work unit "AUTH-002" with a feature file
    // @step And the feature file has a scenario "Password reset flow" with steps:
    // @step And I have a test file "src/__tests__/password-reset.test.ts" with test code
    // @step But the test file is missing step comments
    let project = Project::new(
        "AUTH-002",
        "story",
        "password-reset",
        "Password reset flow",
        &RESET_STEPS,
        "src/__tests__/password-reset.test.ts",
        40,
    )?;

    // @step When I run "fspec link-coverage password-reset --scenario 'Password reset flow' --test-file src/__tests__/password-reset.test.ts --test-lines 20-35"
    let result = project.link_coverage(json!({
        "featureName": "password-reset",
        "scenario": "Password reset flow",
        "testFile": "src/__tests__/password-reset.test.ts",
        "testLines": "20-35"
    }));

    // @step Then the command should fail with exit code 1
    let error = failure(&result)?;

    // @step And the output should contain a system-reminder showing missing step comments
    // @step And the system-reminder should show exact step text to add with "// @step" prefix
    // @step And the system-reminder should NOT mention --skip-step-validation flag
    // @step And the system-reminder should emphasize "Step validation is MANDATORY for story work units"
    assert_strict_missing_step_reminder(&error, &RESET_STEPS, "story")
}

const TIMEOUT_STEPS: [&str; 3] = [
    "Given the login service is slow to respond",
    "When the timeout threshold is exceeded",
    "Then the user should see a timeout error",
];

#[test]
fn bug_work_unit_with_missing_step_comments_receives_strict_system_reminder_without_skip_option(
) -> TestResult {
    // @step Given I have a bug work unit "BUG-045" with a feature file
    // @step And the feature file has a scenario "Fix login timeout" with steps:
    // @step And I have a test file "src/__tests__/login-timeout.test.ts" with test code
    // @step But the test file is missing step comments
    let project = Project::new(
        "BUG-045",
        "bug",
        "fix-login-timeout",
        "Fix login timeout",
        &TIMEOUT_STEPS,
        "src/__tests__/login-timeout.test.ts",
        30,
    )?;

    // @step When I run "fspec link-coverage fix-login-timeout --scenario 'Fix login timeout' --test-file src/__tests__/login-timeout.test.ts --test-lines 10-25"
    let result = project.link_coverage(json!({
        "featureName": "fix-login-timeout",
        "scenario": "Fix login timeout",
        "testFile": "src/__tests__/login-timeout.test.ts",
        "testLines": "10-25"
    }));

    // @step Then the command should fail with exit code 1
    let error = failure(&result)?;

    // @step And the output should contain a system-reminder showing missing step comments
    // @step And the system-reminder should show exact step text to add with "// @step" prefix
    // @step And the system-reminder should NOT mention --skip-step-validation flag
    // @step And the system-reminder should emphasize "Step validation is MANDATORY for bug work units"
    assert_strict_missing_step_reminder(&error, &TIMEOUT_STEPS, "bug")
}

/// Control: the same missing steps on a task work unit do offer the skip
/// flag, so the strict reminders above are type-dependent.
#[test]
fn a_task_work_unit_missing_step_reminder_offers_the_skip_flag() -> TestResult {
    let project = Project::new(
        "TASK-002",
        "task",
        "task-setup",
        "Task setup",
        &RESET_STEPS,
        "src/__tests__/task.test.ts",
        10,
    )?;
    let result = project.link_coverage(json!({
        "featureName": "task-setup",
        "scenario": "Task setup",
        "testFile": "src/__tests__/task.test.ts",
        "testLines": "1-10"
    }));
    let error = failure(&result)?;
    assert!(error.contains(SKIP_SUGGESTION), "{error}");
    assert!(!error.contains("Step validation is MANDATORY"), "{error}");
    Ok(())
}
