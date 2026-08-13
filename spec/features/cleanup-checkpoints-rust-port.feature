@done
@RPC-203
@wip
Feature: Port cleanup-checkpoints command to Rust
  """
  File layout: core rust/fspec-core/src/commands/cleanup_checkpoints.rs is the single source
  of truth (run(args_json, project_root)); CLI bridge rust/fspec/src/cleanup_checkpoints.rs
  parses the workUnitId positional and the required --keep-last flag; help config
  rust/fspec-core/src/help/configs/cleanup_checkpoints.rs.

  codelet_git wiring: reuses the list+sort logic shape from list_checkpoints.rs (read
  .git/fspec-checkpoints-index/<wu>.json, codelet_git::ghost_commit::list_ghost_checkpoints,
  sort by timestamp descending), slices preserved (first keepLast) and deleted (remainder),
  then deletes via codelet_git::ghost_commit::delete_ghost_checkpoint (delete errors swallowed).
  The metadata index is NOT pruned here (parity with TS cleanupCheckpoints; stale entries are
  tolerated and intersected against live refs by list-checkpoints).

  IPC no-op: TS calls sendIPCMessage({type:'checkpoint-changed'}); intentionally omitted.
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to delete old checkpoints for a work unit while keeping the most recent N
    So that I can manage checkpoint retention and avoid accumulating stale save points

  Scenario: Delete the oldest checkpoints beyond the keepLast window
    Given a git repository with 12 checkpoints for work unit "AUTH-001"
    And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 5
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then the result reports deletedCount 7 and preservedCount 5
    And the 5 preserved checkpoints are the 5 newest by timestamp

  Scenario: Render the cleanup summary text
    Given a git repository with 3 checkpoints for work unit "AUTH-001"
    And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 1
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then the rendered text contains "Cleaning up checkpoints for AUTH-001 (keeping last 1)"
    And the rendered text contains "Deleted 2 checkpoint(s):"
    And the rendered text contains "Preserved 1 checkpoint(s):"
    And the rendered text contains "✓ Cleanup complete: 2 deleted, 1 preserved"

  Scenario: No deletion when count is within the keepLast window
    Given a git repository with 3 checkpoints for work unit "BUG-003"
    And the dispatcher receives command "cleanup-checkpoints" with workUnitId "BUG-003" and keepLast 10
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then the result reports deletedCount 0 and preservedCount 3
    And the rendered text contains "✓ Cleanup complete: 0 deleted, 3 preserved"

  Scenario: Reject a keepLast of zero
    Given the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001" and keepLast 0
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then it returns an InvalidArgs error containing "--keep-last must be a positive number"

  Scenario: Reject a missing work unit id
    Given the dispatcher receives command "cleanup-checkpoints" with no workUnitId field and keepLast 5
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then it returns an InvalidArgs error naming the missing workUnitId field

  Scenario: format json emits the structured cleanup payload
    Given a git repository with 4 checkpoints for work unit "AUTH-001"
    And the dispatcher receives command "cleanup-checkpoints" with workUnitId "AUTH-001", keepLast 2 and format "json"
    When fspec_core::commands::cleanup_checkpoints::run executes
    Then the rendered output is pretty-printed JSON
    And it has the keys "workUnitId", "deletedCount", "preservedCount", "deleted", "preserved" in that order
    And "deleted" and "preserved" are arrays of objects with "name" and "timestamp" fields
