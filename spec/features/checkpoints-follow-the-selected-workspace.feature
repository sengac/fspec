@done
@RPC-453
@rpc
@checkpoint-management
@rust
Feature: Checkpoints view and checkpoint counts follow the selected workspace
  """
  Checkpoint list, diff, file diff, restore file, restore all, delete one,
  delete all and the board header checkpoint counts all ran against the
  daemon's start workspace, so restoring a checkpoint while another
  workspace was selected acted on the wrong repository; several TUI flows
  also logged failures and showed an empty list, and checkpoint_counts
  turned a git failure into zero counts.
  New FspecService methods (appended per RPC-449): checkpoint_counts,
  list_checkpoints, checkpoint_diff_files, checkpoint_file_diff,
  restore_checkpoint_file, restore_checkpoint_all, delete_checkpoint and
  delete_all_checkpoints, each with an _in_workspace suffix and a leading
  workspace argument, open the named workspace on the daemon and propagate
  errors. The TUI checkpoint flows and the board counts go through
  app/checkpoint_calls.rs, which uses the board's selected workspace, and
  show failures as an error dialog.
  Tests: rust/fspec-tui/tests/rpc453_checkpoints_follow_the_selected_workspace.rs.
  """

  Background: User Story
    As a user managing checkpoints while another workspace is selected
    I want to list, diff, restore and delete the selected workspace's checkpoints
    So that a restore or delete never acts on the wrong repository

  Scenario: Checkpoints over RPC come from the named workspace
    Given a daemon started in git repository "alpha" and a second git repository "beta" with checkpoint "cp-beta"
    When the checkpoints of workspace "beta" are listed over RPC
    Then the listing contains "cp-beta"
    And the checkpoint counts of workspace "beta" report one manual checkpoint

  Scenario: Restoring a checkpoint over RPC acts on the named workspace
    Given a daemon started in git repository "alpha" and a second git repository "beta" with checkpoint "cp-beta" whose marker file was deleted
    When checkpoint "cp-beta" of workspace "beta" is restored over RPC
    Then the marker file exists again in "beta"

  Scenario: The Checkpoints view lists the selected workspace's checkpoints
    Given the board has workspace "beta" selected
    When the Checkpoints view opens
    Then the backend is asked for the checkpoints of workspace "beta"
