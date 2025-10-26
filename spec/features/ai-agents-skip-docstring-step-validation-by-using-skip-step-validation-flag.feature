@done
@coverage-tracking
@step-validation
@cli
@validation
@critical
@BUG-044
Feature: AI agents skip docstring step validation by using --skip-step-validation flag
  """
  Must detect work unit type by looking up the feature file tag (e.g., @BUG-044) in work-units.json to determine if it's a story, bug, or task. If feature file doesn't exist or tag not found, assume story/bug (strictest validation).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The --skip-step-validation flag must ONLY be available when linking coverage for task work units
  #   2. For story and bug work units, docstring step validation is MANDATORY with NO skip option
  #   3. If --skip-step-validation is used on a story/bug work unit, the command must fail with a strict error message
  #   4. Error message must clearly warn that skipping validation will be detected and require going back to fix docstrings
  #   5. All documentation referencing --skip-step-validation must be updated to explain the task-only restriction
  #   6. Step validation error system-reminders must be updated to remove mention of --skip-step-validation for story/bug work units
  #
  # EXAMPLES:
  #   1. AI agent tries 'fspec link-coverage user-login --scenario Login --test-file auth.test.ts --test-lines 10-20 --skip-step-validation' on story work unit → command fails with error explaining skip is only for tasks
  #   2. AI agent links coverage for task work unit with 'fspec link-coverage infrastructure-setup --scenario Setup --test-file setup.test.ts --test-lines 5-15 --skip-step-validation' → command succeeds (tasks don't require feature files)
  #   3. AI agent links coverage for story without step comments in test file → receives system-reminder with exact steps to add, NO mention of skip flag
  #   4. Documentation shows --skip-step-validation with note: 'Only available for task work units. Story and bug work units require mandatory step validation.'
  #
  # ========================================
  Background: User Story
    As a developer using fspec with AI agents
    I want to enforce mandatory docstring step validation for test-to-scenario traceability
    So that AI agents cannot skip critical validation and bypass the ACDD workflow discipline

  Scenario: Attempt to skip step validation for story work unit fails with strict error
    Given I have a story work unit "AUTH-001" with a feature file
    And the feature file has a scenario "Login with valid credentials"
    And I have a test file "src/__tests__/auth.test.ts" with test code but missing step comments
    When I run "fspec link-coverage user-login --scenario 'Login with valid credentials' --test-file src/__tests__/auth.test.ts --test-lines 10-20 --skip-step-validation"
    Then the command should fail with exit code 1
    And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    And the error message should contain "Story and bug work units require MANDATORY step validation"
    And the error message should warn "Attempting to skip will require going back to fix docstrings when detected"
    And the error message should NOT suggest using --skip-step-validation flag

  Scenario: Attempt to skip step validation for bug work unit fails with strict error
    Given I have a bug work unit "BUG-044" with a feature file
    And the feature file has a scenario "Fix validation bypass"
    And I have a test file "src/__tests__/validation.test.ts" with test code but missing step comments
    When I run "fspec link-coverage ai-agents-skip-docstring-step-validation-by-using-skip-step-validation-flag --scenario 'Fix validation bypass' --test-file src/__tests__/validation.test.ts --test-lines 5-15 --skip-step-validation"
    Then the command should fail with exit code 1
    And the error message should contain "skip-step-validation flag is ONLY allowed for task work units"
    And the error message should contain "Bug work units require MANDATORY step validation"

  Scenario: Skip step validation for task work unit succeeds
    Given I have a task work unit "TASK-001" with a feature file
    And the feature file has a scenario "Setup infrastructure"
    And I have a test file "src/__tests__/setup.test.ts" with test code but missing step comments
    When I run "fspec link-coverage infrastructure-setup --scenario 'Setup infrastructure' --test-file src/__tests__/setup.test.ts --test-lines 5-15 --skip-step-validation"
    Then the command should succeed with exit code 0
    And the output should confirm "Coverage linked successfully"
    And the output should contain a warning "⚠️  Step validation skipped (task work unit)"

  Scenario: Story work unit with missing step comments receives strict system-reminder without skip option
    Given I have a story work unit "AUTH-002" with a feature file
    And the feature file has a scenario "Password reset flow" with steps:
      | Given I am on the password reset page        |
      | When I enter my email address                |
      | Then I should receive a password reset email |
    And I have a test file "src/__tests__/password-reset.test.ts" with test code
    But the test file is missing step comments
    When I run "fspec link-coverage password-reset --scenario 'Password reset flow' --test-file src/__tests__/password-reset.test.ts --test-lines 20-35"
    Then the command should fail with exit code 1
    And the output should contain a system-reminder showing missing step comments
    And the system-reminder should show exact step text to add with "// @step" prefix
    And the system-reminder should NOT mention --skip-step-validation flag
    And the system-reminder should emphasize "Step validation is MANDATORY for story work units"

  Scenario: Bug work unit with missing step comments receives strict system-reminder without skip option
    Given I have a bug work unit "BUG-045" with a feature file
    And the feature file has a scenario "Fix login timeout" with steps:
      | Given the login service is slow to respond |
      | When the timeout threshold is exceeded     |
      | Then the user should see a timeout error   |
    And I have a test file "src/__tests__/login-timeout.test.ts" with test code
    But the test file is missing step comments
    When I run "fspec link-coverage fix-login-timeout --scenario 'Fix login timeout' --test-file src/__tests__/login-timeout.test.ts --test-lines 10-25"
    Then the command should fail with exit code 1
    And the output should contain a system-reminder showing missing step comments
    And the system-reminder should show exact step text to add with "// @step" prefix
    And the system-reminder should NOT mention --skip-step-validation flag
    And the system-reminder should emphasize "Step validation is MANDATORY for bug work units"
