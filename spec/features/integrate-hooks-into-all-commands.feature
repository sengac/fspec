@done
@workflow
@cli
@workflow-automation
@hooks
@phase1
@HOOK-008
Feature: Integrate hooks into all commands

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec CLI user
  #   I want to have hooks execute automatically with every command
  #   So that I can enforce policies and automations without manually invoking hooks
  #
  # BUSINESS RULES:
  #   1. Commands execute pre- hooks before main logic and post- hooks after main logic
  #   2. Hook event names follow pattern: pre-{command} and post-{command}
  #   3. Hooks receive context with workUnitId (if applicable), event name, and timestamp
  #   4. Blocking hook failures prevent command execution (pre-hooks) or prevent exit with success (post-hooks)
  #   5. Non-blocking hook failures do not prevent command execution or success
  #   6. Hooks are filtered by conditions before execution (tags, prefix, epic, estimate)
  #   7. Hook output is displayed using formatHookOutput() with system-reminders for blocking failures
  #
  # EXAMPLES:
  #   1. Command update-work-unit-status triggers pre-update-work-unit-status and post-update-work-unit-status hooks
  #   2. Pre-hook validates work unit has feature file before allowing status change to testing
  #   3. Post-hook runs tests after implementing status change
  #   4. Blocking pre-hook failure prevents command from executing
  #   5. Non-blocking post-hook failure allows command to succeed
  #   6. Hook with tag condition only runs for work units with matching tag
  #   7. Hook output formatted with system-reminder shows blocking hook failure details
  #
  # ========================================
  Background: User Story
    As a fspec CLI user
    I want to have hooks execute automatically with every command
    So that I can enforce policies and automations without manually invoking hooks

  Scenario: Command triggers pre and post hooks
    Given I have hooks configured for "pre-update-work-unit-status" and "post-update-work-unit-status"
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the pre-update-work-unit-status hooks should execute before status change
    And the post-update-work-unit-status hooks should execute after status change
    And the hooks should receive context with workUnitId "AUTH-001"
    And the hooks should receive context with event name
    And the hooks should receive context with timestamp

  Scenario: Pre-hook validates preconditions and blocks command
    Given I have a blocking pre-hook "validate-feature-file" for "pre-update-work-unit-status"
    And the hook checks that work unit has a linked feature file
    And work unit "AUTH-001" has no linked feature file
    When I run "fspec update-work-unit-status AUTH-001 testing"
    Then the pre-hook should execute and fail
    And the command should not execute
    And the hook stderr should be wrapped in system-reminder tags
    And the command should exit with non-zero code

  Scenario: Post-hook runs automation after command succeeds
    Given I have a non-blocking post-hook "run-tests" for "post-update-work-unit-status"
    And work unit "AUTH-001" is being moved to implementing status
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the command should execute successfully
    And the post-hook should execute after status change
    And the hook should receive context with new status "implementing"

  Scenario: Blocking pre-hook failure prevents command execution
    Given I have a blocking pre-hook "lint" for "pre-update-work-unit-status"
    And the hook exits with code 1
    And the hook outputs "Lint errors found" to stderr
    When I run "fspec update-work-unit-status AUTH-001 testing"
    Then the pre-hook should execute and fail
    And the command should not execute
    And the hook stderr should be wrapped in system-reminder tags
    And the system-reminder should contain "Hook: lint"
    And the system-reminder should contain "Exit code: 1"
    And the system-reminder should contain "Lint errors found"

  Scenario: Non-blocking post-hook failure does not affect command success
    Given I have a non-blocking post-hook "notify" for "post-update-work-unit-status"
    And the hook exits with code 1
    And the hook outputs "Notification failed" to stderr
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the command should execute successfully
    And the post-hook should execute after status change
    And the hook failure should not prevent command success
    And the hook stderr should be displayed without system-reminder wrapping

  Scenario: Hook with tag condition only runs for matching work units
    Given I have a hook with condition tags ["@security"]
    And the hook is configured for "post-update-work-unit-status"
    And work unit "AUTH-001" has tags ["@security", "@critical"]
    And work unit "DASH-001" has tags ["@ui", "@phase1"]
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the hook should execute because work unit has @security tag
    When I run "fspec update-work-unit-status DASH-001 implementing"
    Then the hook should not execute because work unit lacks @security tag

  Scenario: Hook output formatted with system-reminder for blocking failures
    Given I have a blocking post-hook "validate" for "post-update-work-unit-status"
    And the hook exits with code 1
    And the hook outputs "Validation failed: tests not passing" to stderr
    When I run "fspec update-work-unit-status AUTH-001 implementing"
    Then the command should execute
    And the post-hook should execute and fail
    And the hook stderr should be wrapped in system-reminder tags
    And the output should contain "<system-reminder>"
    And the output should contain "Hook: validate"
    And the output should contain "Exit code: 1"
    And the output should contain "Validation failed: tests not passing"
    And the output should contain "</system-reminder>"
