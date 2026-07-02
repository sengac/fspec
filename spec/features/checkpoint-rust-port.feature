@done
@RPC-202
@wip
Feature: Port checkpoint command to Rust
  """
  File layout: core codelet/fspec-core/src/commands/checkpoint.rs is the single source of
  truth (run(args_json, project_root)); CLI bridge codelet/fspec/src/checkpoint.rs marshals
  the two positionals into JSON; help config codelet/fspec-core/src/help/configs/checkpoint.rs.

  codelet_git wiring: calls codelet_git::ghost_commit::create_ghost_commit(project_root,&wu,&name);
  an empty files vec => success:false. The .git/fspec-checkpoints-index/<wu>.json metadata write
  lives in fspec-core (codelet-git does NOT touch the index), pretty-printed with 2-space indent
  matching JSON.stringify(...,null,2).

  IPC no-op: TS calls sendIPCMessage({type:'checkpoint-changed'}); the Rust dispatcher has no TUI
  IPC channel so this is intentionally omitted (no-op).
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to create a named manual checkpoint capturing my current working-tree changes
    So that I have a rollback point before risky experiments

  Scenario: Capture a dirty working tree and render the success banner
    Given a git repository with 3 uncommitted file changes
    And the dispatcher receives command "checkpoint" with workUnitId "AUTH-001" and checkpointName "baseline"
    When fspec_core::commands::checkpoint::run executes with project_root set to that repository
    Then the result succeeds
    And the rendered text contains "✓ Created checkpoint \"baseline\" for AUTH-001"
    And the rendered text contains "Captured 3 file(s)"

  Scenario: Persist the metadata index on successful capture
    Given a git repository with uncommitted changes
    When fspec_core::commands::checkpoint::run captures a checkpoint named "baseline" for "AUTH-001"
    Then the file ".git/fspec-checkpoints-index/AUTH-001.json" exists
    And it contains a checkpoints entry whose name is "baseline" with a sha and an ISO-8601 timestamp
    And the JSON is pretty-printed with 2-space indentation

  Scenario: Clean working tree captures nothing and reports failure
    Given a git repository with no uncommitted changes
    When fspec_core::commands::checkpoint::run attempts to capture "baseline" for "AUTH-001"
    Then the result reports success false with an empty capturedFiles list
    And no ".git/fspec-checkpoints-index/AUTH-001.json" file is written

  Scenario: Reject an empty checkpoint name
    Given the dispatcher receives command "checkpoint" with workUnitId "AUTH-001" and an empty checkpointName
    When fspec_core::commands::checkpoint::run executes
    Then it returns an InvalidArgs error naming the empty checkpointName field

  Scenario: Reject a missing work unit id
    Given the dispatcher receives command "checkpoint" with no workUnitId field
    When fspec_core::commands::checkpoint::run executes
    Then it returns an InvalidArgs error naming the missing workUnitId field

  Scenario: format json emits the structured payload preserving key order
    Given a git repository with uncommitted changes
    And the dispatcher receives command "checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and format "json"
    When fspec_core::commands::checkpoint::run executes
    Then the rendered output is pretty-printed JSON
    And the object keys are "success", "checkpointName", "capturedFiles", "includedUntracked" in that order
    And "includedUntracked" is true
