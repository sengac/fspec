@cli
@hook-execution
@critical
@hooks
@workflow-automation
@bug
@HOOK-012
Feature: Virtual hooks don't execute during status transitions
  """
  Architecture notes:
  - Virtual hooks are stored in work-units.json but currently never execute
  - update-work-unit-status.ts does NOT call executeVirtualHooks function
  - Fix requires integration of execa library for command execution
  - Blocking hooks must wrap errors in <system-reminder> tags for AI visibility
  - Hook execution must happen BEFORE status transition (pre-) or AFTER (post-)
  - executeVirtualHooks utility already exists in src/hooks/executor.ts but is not called
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Virtual hooks MUST execute during status transitions using execa library
  #   2. update-work-unit-status command MUST call executeVirtualHooks before status change
  #   3. Blocking hook failures MUST prevent status transitions and wrap errors in <system-reminder> tags
  #
  # EXAMPLES:
  #   1. Work unit has blocking virtual hook 'exit 1' at pre-validating. When moving to validating, hook executes and fails with exit code 1. Transition is blocked and stderr contains <system-reminder>BLOCKING HOOK FAILURE</system-reminder>.
  #   2. Work unit has passing hook 'echo success' at pre-validating. When moving to validating, hook executes successfully with exit code 0. Transition proceeds and status becomes validating.
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec workflow
    I want to have virtual hooks execute during status transitions
    So that I can enforce quality gates and blocking checks for specific work units

  Scenario: Blocking virtual hook fails and prevents status transition
    Given work unit "TEST-001" is in implementing status
    And work unit has a blocking virtual hook "exit 1" at pre-validating event
    When I run "fspec update-work-unit-status TEST-001 validating"
    Then the virtual hook MUST execute before the transition
    And the hook MUST use execa library for execution
    And the command "exit 1" should fail with exit code 1
    And the transition MUST be blocked
    And stderr MUST contain "<system-reminder>" tags
    And stderr MUST contain "BLOCKING HOOK FAILURE"
    And work unit status MUST remain "implementing"

  Scenario: Passing virtual hook allows status transition
    Given work unit "TEST-001" is in implementing status
    And work unit has a blocking virtual hook "echo success" at pre-validating event
    When I run "fspec update-work-unit-status TEST-001 validating"
    Then the virtual hook MUST execute successfully
    And the command "echo success" should succeed with exit code 0
    And the transition MUST succeed
    And work unit status MUST be "validating"
