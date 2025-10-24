@work-management
@workflow
@kanban
@cli
@high
@BOARD-004
Feature: Work unit ordering across all Kanban columns

  """
  Done column is exempt from manual ordering (sorted by completion timestamp). Attempts to prioritize done work units emit system-reminders explaining the constraint and listing allowed columns for AI agent guidance.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Currently: prioritize-work-unit only works for backlog status, throws error for other statuses
  #   2. Change: Allow prioritize-work-unit to work on backlog, specifying, testing, implementing, validating, blocked statuses (NOT done)
  #   3. CRITICAL: prioritize-work-unit can ONLY reorder within the same column (same status). Cannot move work units between columns or reference work units in different columns.
  #   4. If --before or --after references a work unit in a different column, throw error with clear message
  #   5. If work unit is in done status, throw error AND emit system-reminder explaining done work units cannot be reordered
  #   6. Files requiring updates: src/commands/prioritize-work-unit.ts (lines 36-41, 91), src/commands/prioritize-work-unit-help.ts (lines 5, 8, 9, 25, 29, examples, commonErrors 66-67, typicalWorkflow 76, notes 96+100), src/help.ts (prioritize section), spec/features/kanban-workflow-state-management.feature (prioritize scenarios and validation)
  #   7. Update command description to: 'Reorder work units in any Kanban column except done'
  #   8. If work unit is the only item in its column, allow prioritization and succeed as no-op (idempotent operation)
  #   9. Update spec/features/kanban-workflow-state-management.feature: modify existing prioritize scenarios to test multi-column prioritization, remove validation scenario that tests 'Can only prioritize work units in backlog state' error (no longer valid)
  #   10. Update test files (src/commands/__tests__/kanban-workflow-state-management.test.ts and others): modify tests to cover multi-column prioritization, remove tests that specifically validate 'backlog-only' constraint (no longer valid)
  #
  # EXAMPLES:
  #   1. User runs 'fspec prioritize-work-unit FEAT-017 --position top' where FEAT-017 is in specifying status. Command reorders FEAT-017 to top of specifying column (states.specifying array).
  #   2. User runs 'fspec prioritize-work-unit FEAT-017 --before AUTH-001' where FEAT-017 is in specifying and AUTH-001 is in testing. Command throws error: 'Cannot prioritize across columns. FEAT-017 (specifying) and AUTH-001 (testing) are in different columns.'
  #   3. User runs 'fspec prioritize-work-unit AUTH-001 --position top' where AUTH-001 is in done status. Command throws error and emits: '<system-reminder>Cannot prioritize work units in done column. Done items are ordered by completion time and cannot be manually reordered. Only backlog, specifying, testing, implementing, validating, blocked can be prioritized.</system-reminder>'
  #   4. Help text shows: 'fspec prioritize-work-unit - Reorder work units in any Kanban column except done'
  #   5. AUTH-001 is only item in implementing column. User runs 'fspec prioritize-work-unit AUTH-001 --position top'. Command succeeds with message: '✓ Work unit AUTH-001 prioritized successfully' (no error, no-op).
  #   6. Old scenario: 'Attempt to prioritize work not in backlog' tests error when implementing work unit is prioritized. REMOVE this scenario (no longer an error). Add new scenarios testing prioritization in specifying, testing, implementing, validating, blocked columns.
  #   7. Old test: 'should throw error when prioritizing work unit in implementing status' - REMOVE (no longer an error). Add new tests: 'should prioritize work unit in specifying column', 'should prioritize work unit in implementing column', etc.
  #
  # ========================================

  Background: User Story
    As a developer managing work priorities
    I want to reorder work units in any Kanban column except done
    So that I can adjust priorities throughout the workflow, not just in backlog

  Scenario: Prioritize work unit within specifying column
    Given work unit FEAT-017 is in specifying status
    And the specifying column contains multiple work units
    When I run "fspec prioritize-work-unit FEAT-017 --position top"
    Then the command should succeed
    And FEAT-017 should be first in the states.specifying array
    And FEAT-017 should remain in specifying status

  Scenario: Prioritize work unit within implementing column
    Given work unit AUTH-001 is in implementing status
    And the implementing column contains multiple work units
    When I run "fspec prioritize-work-unit AUTH-001 --position top"
    Then the command should succeed
    And AUTH-001 should be first in the states.implementing array
    And AUTH-001 should remain in implementing status

  Scenario: Error when prioritizing across columns using --before
    Given work unit FEAT-017 is in specifying status
    And work unit AUTH-001 is in testing status
    When I run "fspec prioritize-work-unit FEAT-017 --before AUTH-001"
    Then the command should fail
    And the error message should contain "Cannot prioritize across columns"
    And the error message should contain "FEAT-017 (specifying) and AUTH-001 (testing) are in different columns"

  Scenario: Error when prioritizing work unit in done column
    Given work unit AUTH-001 is in done status
    When I run "fspec prioritize-work-unit AUTH-001 --position top"
    Then the command should fail
    And the error message should contain "Cannot prioritize work units in done column"
    And a system-reminder should be emitted
    And the system-reminder should explain that done items cannot be manually reordered
    And the system-reminder should list allowed columns: backlog, specifying, testing, implementing, validating, blocked

  Scenario: Single work unit in column succeeds as no-op
    Given work unit AUTH-001 is the only work unit in implementing status
    When I run "fspec prioritize-work-unit AUTH-001 --position top"
    Then the command should succeed
    And the success message should show "Work unit AUTH-001 prioritized successfully"
    And AUTH-001 should remain the only work unit in implementing status

  Scenario: Command description updated to reflect multi-column support
    When I run "fspec prioritize-work-unit --help"
    Then the help text should contain "Reorder work units in any Kanban column except done"
    And the help text should NOT contain "in the backlog" only
