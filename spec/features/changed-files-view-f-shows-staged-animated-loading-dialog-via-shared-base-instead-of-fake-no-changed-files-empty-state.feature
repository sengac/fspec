@wip
@TUI-108
Feature: Changed Files view ('f') shows staged animated loading dialog via shared base instead of fake 'No changed files' empty state

  """
  UI/UX:
  - UI/UX: Same shared LoadingDialog/LoadTracker as TUI-106 (extending the shared base dialog); ChangedFiles-specific: two-stage labels keyed 'list' and 'diff:{path}', mounted on view construction, painted over both panes; stale FileDiffLoaded for a de-selected path must not clear the current diff-stage loading flag (preserves existing diff_path stale-drop).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While the changed_files RPC is in flight, the body shows the shared LoadingDialog ('Loading changed files…') and never the 'No changed files' empty message; 'No changed files' appears only after the scan completes with zero entries
  #   2. While any file's diff is being loaded (initial selection or an arrow-key selection change), the dialog shows 'Loading diff for <path>…' with the spinner until that diff folds in; the last diff result wins and stale results for other paths are dropped without clearing the current stage's loading flag
  #
  # EXAMPLES:
  #   1. I press 'f' on a dirty tree; instead of the view lying with 'No changed files' I see 'Loading changed files…' with a spinner, then the changed-file list appears
  #   2. I arrow down to a second changed file; the right pane shows 'Loading diff for src/foo.rs…' with the spinner until the diff appears, and quickly arrowing through files never shows a stale diff for a file I moved away from
  #
  # ========================================

  Background: User Story
    As a fspec TUI user on the board
    I want to open the Changed Files view and see a real loading dialog instead of 'No changed files' while the tree is scanned
    So that know the view is working on a dirty tree and can tell a clean tree from a slow scan
