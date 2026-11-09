@BUG-582
@done
@tui
@ui-refinement
@TUI-109
@BUG-355
Feature: Checkpoint enumeration streams per-item progress frames
  """
  Split out of stream-per-item-progress-idx-total-for-checkpoint-enumeration-into-the-shared-loadingdialog-counter-row.feature by BUG-355 so each feature maps to exactly one test file (1 feature file = 1 test file). These scenarios pin the git and rpc halves of TUI-109: git's list_all_ghost_checkpoints_stream ticks a callback per checkpoint ref, rpc's collect_checkpoints_stream shapes the ticks into CheckpointsProgress{loaded,total,done} frames (loaded stops at the 200-entry cap while total reflects the full enumeration), FspecServiceImpl::list_checkpoints publishes the frames on the shared broadcast, and the non-streaming entry points delegate to the streaming ones with a no-op callback. The TUI fold scenarios stay in the original feature.
  Test file: rust/fspec-tui/tests/tui109_checkpoint_enumeration_stream.rs (repo helpers shared through tests/common/checkpoint_repo.rs).
  """

  Background: User Story
    As a fspec TUI user on a repo with many checkpoints
    I want the server-side checkpoint enumeration to report progress per item without changing the CLI output
    So that the loading dialog has a real counter to show while the CLI stays byte-identical

  Scenario: Capped enumeration shows (200/250) - truncation does not hide progress
    Given a repository with 250 checkpoints
    When the streaming enumeration collects items with the 200-entry cap applied
    Then the progress counter reaches (200/250) - loaded stops at the cap while the total reflects the full enumeration
    And the returned list contains exactly 200 entries

  Scenario: The non-streaming CLI path is byte-identical
    Given the non-streaming collect_checkpoints delegates to the streaming variant with a no-op callback
    When the CLI list-checkpoints command runs against a repository with checkpoints
    Then the output is byte-identical to the pre-streaming behavior
    And the existing list_checkpoints tests pass unmodified
