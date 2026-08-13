@done
@RPC-288
@wip
Feature: Port restore-checkpoint command to Rust
  """
  File layout: core rust/fspec-core/src/commands/restore_checkpoint.rs is the single source
  of truth (run(args_json, project_root)); CLI bridge rust/fspec/src/restore_checkpoint.rs
  marshals the two positionals (force/userChoice are NOT exposed on the clap surface, parity with
  the TS Commander.js registration); help config rust/fspec-core/src/help/configs/restore_checkpoint.rs.

  codelet_git wiring: dirty check via codelet_git::{get_staged_files,get_unstaged_files,
  get_untracked_files} (any error => treated as not dirty); conflict pre-check via
  codelet_git::ghost_commit::get_checkpoint_diff_files; restore via
  codelet_git::ghost_commit::restore_ghost_commit(project_root,&wu,&name,force). A ref-not-found
  Err maps to success:false plus the 'Checkpoint "<name>" not found for work unit <wu>'
  systemReminder (NOT an InvalidArgs error — parity with the TS catch).

  System-reminder text must be byte-identical to the TS util's CHECKPOINT RESTORATION CONFLICT
  DETECTED block (see ast-research-restore-checkpoint.md). restore has NO IPC call in TS.
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to restore a previously created checkpoint back into my working directory with conflict detection
    So that I can revert to a known-good state after a failed experiment

  Scenario: Restore against a clean working tree
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and checkpointName "baseline"
    When fspec_core::commands::restore_checkpoint::run executes
    Then the result reports success true and conflictsDetected false
    And the rendered text contains "✓ Restored checkpoint \"baseline\" for AUTH-001"

  Scenario: Dirty working tree without a choice shows the risk options and requires a user choice
    Given a git repository with a checkpoint "before-refactor" for "UI-002" and uncommitted changes
    And the dispatcher receives command "restore-checkpoint" with workUnitId "UI-002" and checkpointName "before-refactor" and no force and no userChoice
    When fspec_core::commands::restore_checkpoint::run executes
    Then the result reports success false and requiresUserChoice true
    And the rendered text contains "Working directory has uncommitted changes"
    And the rendered text lists three numbered risk options including "Low", "Medium", and "High"
    And no files are restored

  Scenario: Conflicts detected when working-tree files differ from the checkpoint
    Given a git repository with a checkpoint "previous-state" for "AUTH-001"
    And working-tree files differ from that checkpoint and the request does not force
    But the request supplies userChoice so the conflict pre-check runs
    When fspec_core::commands::restore_checkpoint::run executes
    Then conflictsDetected is true and conflictedFiles lists the differing files
    And the systemReminder contains "CHECKPOINT RESTORATION CONFLICT DETECTED"
    And the systemReminder ends with "</system-reminder>"

  Scenario: Missing checkpoint ref reports not-found without erroring
    Given a git repository with no checkpoint named "ghost" for "AUTH-001"
    And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and checkpointName "ghost"
    When fspec_core::commands::restore_checkpoint::run executes
    Then the result reports success false
    And the systemReminder is "Checkpoint \"ghost\" not found for work unit AUTH-001"

  Scenario: Reject an empty checkpoint name
    Given the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and an empty checkpointName
    When fspec_core::commands::restore_checkpoint::run executes
    Then it returns an InvalidArgs error naming the empty checkpointName field

  Scenario: force restore against a dirty repo succeeds without conflicts
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and uncommitted changes
    And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and force true
    When fspec_core::commands::restore_checkpoint::run executes
    Then the result reports success true and conflictsDetected false

  Scenario: format json emits the structured payload preserving key order
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and format "json"
    When fspec_core::commands::restore_checkpoint::run executes
    Then the rendered output is pretty-printed JSON
    And it has the keys "success", "conflictsDetected", "conflictedFiles", "systemReminder", "requiresTestValidation" in that order
