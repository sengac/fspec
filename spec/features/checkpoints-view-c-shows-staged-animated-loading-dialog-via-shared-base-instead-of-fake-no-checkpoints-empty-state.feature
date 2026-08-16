@wip
@TUI-107
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
