@done
@checkpoint-management
@git
@rust
@RPC-362
Feature: Checkpoint transport methods (list + diff-files + file-diff + restore + delete)
  """
  Adds 7 FspecBackend transport methods (list_checkpoints, checkpoint_diff_files, checkpoint_file_diff, restore_checkpoint_file, restore_checkpoint_all, delete_checkpoint, delete_all_checkpoints) mirroring the checkpoint_counts plumbing across the trait, embedded + websocket transports, and the tarpc FspecService.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. list_checkpoints returns checkpoints sorted most-recent-first and capped at 200
  #   2. Each CheckpointInfo marks is_automatic true when its name contains -auto-
  #   3. checkpoint_diff_files and checkpoint_file_diff delegate to the existing ghost_commit/git helpers and return ChangedFile list and Option<String> diff respectively
  #   4. restore and delete methods delegate to ghost_commit helpers and propagate errors via Result with no panic in production paths
  #   5. Every new transport method is implemented on the trait, the embedded transport, the websocket transport, and the tarpc service
  #
  # EXAMPLES:
  #   1. A repo with 3 checkpoints (two auto, one manual) returns 3 CheckpointInfo most-recent-first with is_automatic flags [true,true,false] by creation order
  #   2. A repo with 250 checkpoints returns exactly 200 CheckpointInfo (the 200 most recent)
  #   3. checkpoint_diff_files for a checkpoint that changed a.txt and b.txt returns two ChangedFile entries
  #   4. restore_checkpoint_all on a checkpoint restores the working tree and returns Ok; a subsequent checkpoint_file_diff shows no diff
  #   5. delete_checkpoint removes one checkpoint so a subsequent list_checkpoints returns one fewer entry
  #
  # ========================================
  Background: User Story
    As a developer
    I want to list, diff, restore and delete git checkpoints from the Rust TUI
    So that the CheckpointsView can display and manage checkpoints without reimplementing git logic

  Scenario: list_checkpoints returns checkpoints most-recent-first with automatic flags
    Given a git repository with three checkpoints created in order baseline, AUTH-001-auto-a, AUTH-001-auto-b
    When I call the list_checkpoints helper against that repository
    Then it returns three CheckpointInfo entries most-recent-first
    And the is_automatic flags are true, true, false in returned order

  Scenario: list_checkpoints caps the result at 200 entries
    Given a git repository with 250 checkpoints
    When I call the list_checkpoints helper against that repository
    Then it returns exactly 200 CheckpointInfo entries

  Scenario: checkpoint_diff_files returns one ChangedFile per changed file
    Given a checkpoint whose working tree differs in a.txt and b.txt
    When I call the checkpoint_diff_files helper for that checkpoint
    Then it returns two ChangedFile entries for a.txt and b.txt

  Scenario: checkpoint_file_diff returns the unified diff for a changed file
    Given a checkpoint whose working tree differs in a.txt
    When I call the checkpoint_file_diff helper for a.txt
    Then it returns Some unified diff text for a.txt

  Scenario: restore_checkpoint_all restores the working tree
    Given a checkpoint and a working tree modified after it
    When I call the restore_checkpoint_all helper for that checkpoint
    Then the call returns Ok
    And a subsequent checkpoint_diff_files reports no changed files

  Scenario: delete_checkpoint removes one checkpoint
    Given a git repository with two checkpoints
    When I call the delete_checkpoint helper for one checkpoint
    Then a subsequent list_checkpoints returns one fewer entry

  Scenario: restore_checkpoint_file restores a single file
    Given a checkpoint and a single file modified after it
    When I call the restore_checkpoint_file helper for a.txt
    Then the file content matches the checkpoint snapshot

  Scenario: delete_all_checkpoints removes every checkpoint
    Given a git repository with three checkpoints
    When I call the delete_all_checkpoints helper
    Then a subsequent list_checkpoints returns no entries
