@done
@RPC-366
@diff-viewer
@tui
Feature: Checkpoint Delete Dispatch
  """
  App::dispatch routes Action::DeleteCheckpoint / Action::DeleteAllCheckpoints through dispatch_checkpoint_delete.rs which spawns backend.delete_checkpoint / backend.delete_all_checkpoints (RPC-362 transport) and folds back Action::DeleteCheckpointResult. Verified with the MockBackend test double (no real git repo required).
  """

  Background: User Story
    As a fspec user confirming a checkpoint delete
    I want the App to dispatch the delete through the transport backend
    So that the checkpoint is actually removed via the RPC layer

  Scenario: Dispatching DeleteCheckpoint calls the transport delete_checkpoint
    Given an App whose backend records delete calls
    When Action::DeleteCheckpoint is dispatched for a checkpoint
    Then the backend delete_checkpoint is called for that checkpoint
    And the App emits a DeleteCheckpointResult action

  Scenario: Dispatching DeleteAllCheckpoints calls the transport delete_all_checkpoints
    Given an App whose backend records delete calls
    When Action::DeleteAllCheckpoints is dispatched
    Then the backend delete_all_checkpoints is called
    And the App emits a DeleteCheckpointResult action
