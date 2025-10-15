@validator
@FEAT-011
@phase1
@workflow
@validation
@acdd
Feature: Prevent retroactive state walking - enforce temporal ordering
  """
  Architecture notes:
  - Uses filesystem mtime (modification time) to determine when files were created/modified
  - Compares file mtime against work unit stateHistory timestamps
  - Blocks state transitions if files were modified BEFORE entering required state
  - Provides --skip-temporal-validation escape hatch for reverse ACDD and importing legacy work
  - Tasks (type='task') are exempt from test file validation since they don't require tests

  Critical implementation requirements:
  - MUST check feature file mtime when moving to testing state
  - MUST check test file mtime when moving to implementing state
  - MUST provide clear error messages showing file time vs state entry time
  - MUST support --skip-temporal-validation flag for legitimate cases
  - Error messages MUST include: file path, file mtime, state entry time, gap duration

  References:
  - FEAT-011 work unit in spec/work-units.json
  - Implementation: src/utils/temporal-validation.ts
  - Integration: src/commands/update-work-unit-status.ts
  """

  Background: User Story
    As an AI agent using fspec for ACDD workflow
    I want to ensure work is done in the correct temporal order
    So that ACDD methodology is actually enforced, not just theatrical state walking

  Scenario: Detect retroactive feature file creation
    Given I have a work unit that entered specifying state at time T1
    And I have a feature file tagged with the work unit ID that was created BEFORE T1
    When I try to move to testing state
    Then the command should fail with temporal ordering violation
    And the error should show that feature files were created BEFORE entering specifying state

  Scenario: Allow valid ACDD workflow
    Given I have a work unit that entered specifying state at time T1
    And I have a feature file tagged with the work unit ID that was created AFTER T1
    When I try to move to testing state
    Then the command should succeed

  Scenario: Detect retroactive test file creation
    Given I have a work unit that entered testing state at time T2
    And I have a test file that references the work unit, created BEFORE T2
    And I have a feature file created after specifying state (to pass other validation)
    When I try to move to implementing state
    Then the command should fail with temporal ordering violation
    And the error should show that test files were created BEFORE entering testing state

  Scenario: Escape hatch with --skip-temporal-validation
    Given I have a work unit with retroactive files (reverse ACDD scenario)
    And the feature file was created BEFORE entering specifying state
    When I use --skip-temporal-validation flag
    Then the command should succeed

  Scenario: Tasks are exempt from test file temporal validation
    Given I have a task work unit (tasks don't require tests)
    And the work unit is in implementing state
    When I move to validating state (skipping testing state for tasks)
    Then the command should succeed without checking for test files
