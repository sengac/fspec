@done
@TUI-107
@ui-refinement
@tui
Feature: Checkpoints view ('c') shows staged animated loading dialog via shared base instead of fake 'No checkpoints' empty state
  """
  UI/UX:
  - UI/UX: The loading dialog reuses the shared LoadingDialog / LoadTracker / spinner / run-loop gate from TUI-106 (mounted on view construction, keyed by stage), painted over the panes the same way the RPC-365 restore modal is; it extends the shared base dialog (dialog_theme), never re-invents one. Checkpoints-specific: three-stage labels keyed 'list' / 'files:{workUnitId}:{name}' / 'diff:{workUnitId}:{name}:{path}'; stale CheckpointFilesLoaded/CheckpointFileDiffLoaded for a no-longer-selected checkpoint must not clear the current stage's loading flag.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While the checkpoint list RPC is in flight, the body shows the shared LoadingDialog ('Loading checkpoint list…') and never the 'No checkpoints available' empty message; 'No checkpoints available' appears only after the list has loaded with zero entries
  #   2. After the list loads, the dialog transitions to per-stage labels ('Loading files for <checkpoint>…', 'Loading diff for <file>…') until each result folds in; the dialog vanishes automatically when a stage flushes
  #   3. A result for a checkpoint/file that is no longer selected is dropped without affecting the current stage's loading state (existing stale-drop keys files_key/diff_key preserved through the shared LoadTracker)
  #
  # EXAMPLES:
  #   1. I press 'c' on a repo with 180 checkpoints; instead of 'No checkpoints available' I see the 'Loading checkpoint list…' dialog with a spinner, then the checkpoint list appears most-recent-first
  #
  # ========================================
  Background: User Story
    As a fspec TUI user on the board
    I want to open the Checkpoints view and see exactly which checkpoint-loading step is running instead of being told there are no checkpoints
    So that stop re-pressing 'c' on large repos because the view looked broken

  Scenario: Opening the Checkpoints view before the list returns shows the loading dialog instead of the empty message
    Given the Checkpoints view is opened
    When the checkpoint list request has not yet returned
    Then the body shows the animated loading dialog with the label "Loading checkpoint list…"
    And the body does not show "No checkpoints available"

  Scenario: A completed list load with zero checkpoints shows the real empty message
    Given the Checkpoints view is opened
    When the checkpoint list request completes with zero checkpoints
    Then the view shows "No checkpoints available"
    And no loading dialog is shown

  Scenario: Selecting a checkpoint after the list loads shows the files stage label
    Given the checkpoint list is loaded with at least one checkpoint
    When a checkpoint is selected
    Then the loading dialog shows the label "Loading files for <checkpoint label>…"

  Scenario: Loading a file diff shows the diff stage label until it folds in
    Given the checkpoint files are loaded with at least one file
    When a file diff load is in flight
    Then the loading dialog shows the label "Loading diff for <file path>…"
    When the diff result folds in
    Then the loading dialog disappears

  Scenario: A stale files result for a de-selected checkpoint does not clear the current stage
    Given the files stage is in flight for the selected checkpoint
    When a files result arrives for a checkpoint that is no longer selected
    Then the current stage's loading state is unchanged

  Scenario: ESC is ignored while the loading dialog is active and closes the view after it flushes
    Given the loading dialog is active
    When the user presses ESC
    Then the view stays open
    When the loading has flushed
    When the user presses ESC
    Then the view emits CloseCheckpointsView

  Scenario: The loading dialog renders through the canonical dialog theme
    Given the loading dialog is active
    When the view is rendered
    Then the dialog shows a rounded border in the cyan accent
    And the dialog title is "Loading checkpoints"
    And the spinner glyph advances between 0 ms and 80 ms
