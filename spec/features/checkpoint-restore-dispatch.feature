@done
@RPC-365
@diff-viewer
@tui
Feature: Checkpoint Restore Dispatch
  """
  App::dispatch routes Action::RestoreCheckpointFile through dispatch_checkpoint_restore.rs which spawns backend.restore_checkpoint_file (RPC-362 transport) and folds back Action::RestoreCheckpointResult. Verified with the MockBackend test double (no real git repo required).
  """

  Background: User Story
    As a fspec user confirming a checkpoint restore
    I want the App to dispatch the restore through the transport backend
    So that the working tree is actually restored via the RPC layer

  Scenario: Dispatching RestoreCheckpointFile calls the transport restore_checkpoint_file
    Given an App whose backend records restore calls
    When Action::RestoreCheckpointFile is dispatched for a.txt
    Then the backend restore_checkpoint_file is called for a.txt
    And the App emits a RestoreCheckpointResult action

  Scenario: Dispatching RestoreCheckpointAll calls the transport restore_checkpoint_all
    Given an App whose backend records restore calls
    When Action::RestoreCheckpointAll is dispatched
    Then the backend restore_checkpoint_all is called once
    And the App emits a RestoreCheckpointResult action
