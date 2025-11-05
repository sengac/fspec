@done
@cli
@bug
@checkpoint
@git
@workflow
@cleanup
@BUG-066
Feature: Auto-checkpoints not cleaned up when work unit moves to done
  """
  Uses cleanupAutoCheckpoints() function in git-checkpoint.ts to filter and delete only automatic checkpoints. Automatic checkpoints identified by naming pattern {workUnitId}-auto-{state}. Manual checkpoints preserved (any name not matching auto pattern). Cleanup triggered after auto-compact when work unit moves to done status in update-work-unit-status.ts. Deletion process: (1) delete git ref at .git/refs/fspec-checkpoints/{workUnitId}/{checkpointName}, (2) remove entry from index file at .git/fspec-checkpoints-index/{workUnitId}.json. Error handling: silently skip cleanup if git operations fail (allows commands to work without git repository).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Auto-checkpoints are created with naming pattern: {workUnitId}-auto-{fromState} (e.g., AUTH-001-auto-specifying)
  #   2. Manual checkpoints are created by user with custom names and should NEVER be deleted automatically
  #   3. When work unit moves to done status, ONLY auto-checkpoints for that work unit should be deleted
  #   4. Checkpoints are stored in git refs (refs/fspec-checkpoints/{workUnitId}/{checkpointName}) and index files (.git/fspec-checkpoints-index/{workUnitId}.json)
  #   5. CURRENT BUG: cleanupCheckpoints() function exists in git-checkpoint.ts but is a STUB that does not actually delete anything (lines 514-515: 'For now, just return the split')
  #   6. CURRENT BUG: cleanupCheckpoints() is NEVER called from update-work-unit-status.ts when work unit moves to done status
  #   7. CURRENT IMPLEMENTATION FLAW: cleanupCheckpoints() uses keepLast parameter which would delete BOTH manual and auto checkpoints - needs to filter by isAutomatic flag instead
  #   8. listCheckpoints() already identifies automatic checkpoints using isAutomatic flag (line 465-467: checks if name starts with {workUnitId}-auto-)
  #   9. To delete checkpoint: (1) delete git ref file at .git/refs/fspec-checkpoints/{workUnitId}/{checkpointName} using fs.unlink, (2) remove entry from index file at .git/fspec-checkpoints-index/{workUnitId}.json
  #   10. FIX LOCATION: In update-work-unit-status.ts after auto-compact completes (around line 489), need to call cleanup function to delete auto-checkpoints for the work unit
  #
  # EXAMPLES:
  #   1. Work unit AUTH-001 has auto-checkpoints: AUTH-001-auto-specifying, AUTH-001-auto-testing, and manual checkpoint: before-major-refactor. When AUTH-001 moves to done, only the 2 auto-checkpoints should be deleted, before-major-refactor should remain
  #   2. Work unit BUG-027 has only manual checkpoints: before-fix, after-tests. When BUG-027 moves to done, no checkpoints should be deleted
  #   3. Work unit FEAT-010 has only auto-checkpoints: FEAT-010-auto-backlog, FEAT-010-auto-specifying. When FEAT-010 moves to done, all auto-checkpoints should be deleted
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to have auto-checkpoints cleaned up when work unit is done
    So that checkpoint storage doesn't accumulate indefinitely and manual checkpoints are preserved

  Scenario: Work unit with mixed auto and manual checkpoints
    Given a work unit "AUTH-001" exists with status "implementing"
    And "AUTH-001" has automatic checkpoint "AUTH-001-auto-specifying"
    And "AUTH-001" has automatic checkpoint "AUTH-001-auto-testing"
    And "AUTH-001" has manual checkpoint "before-major-refactor"
    When I run "fspec update-work-unit-status AUTH-001 done"
    Then the command should succeed
    And checkpoint "AUTH-001-auto-specifying" should be deleted
    And checkpoint "AUTH-001-auto-testing" should be deleted
    And checkpoint "before-major-refactor" should exist
    And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-specifying" should not exist
    And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing" should not exist
    And the git ref "refs/fspec-checkpoints/AUTH-001/before-major-refactor" should exist
    And the index file should not contain "AUTH-001-auto-specifying"
    And the index file should not contain "AUTH-001-auto-testing"
    And the index file should contain "before-major-refactor"

  Scenario: Work unit with only manual checkpoints
    Given a work unit "BUG-027" exists with status "validating"
    And "BUG-027" has manual checkpoint "before-fix"
    And "BUG-027" has manual checkpoint "after-tests"
    When I run "fspec update-work-unit-status BUG-027 done"
    Then the command should succeed
    And checkpoint "before-fix" should exist
    And checkpoint "after-tests" should exist
    And the git ref "refs/fspec-checkpoints/BUG-027/before-fix" should exist
    And the git ref "refs/fspec-checkpoints/BUG-027/after-tests" should exist
    And no checkpoints should be deleted

  Scenario: Work unit with only automatic checkpoints
    Given a work unit "FEAT-010" exists with status "validating"
    And "FEAT-010" has automatic checkpoint "FEAT-010-auto-backlog"
    And "FEAT-010" has automatic checkpoint "FEAT-010-auto-specifying"
    When I run "fspec update-work-unit-status FEAT-010 done"
    Then the command should succeed
    And checkpoint "FEAT-010-auto-backlog" should be deleted
    And checkpoint "FEAT-010-auto-specifying" should be deleted
    And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-backlog" should not exist
    And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-specifying" should not exist
    And all checkpoints for "FEAT-010" should be deleted

  Scenario: Work unit with no checkpoints
    Given a work unit "TASK-001" exists with status "implementing"
    And "TASK-001" has no checkpoints
    When I run "fspec update-work-unit-status TASK-001 done"
    Then the command should succeed
    And no errors should occur
    And no checkpoints should be deleted
