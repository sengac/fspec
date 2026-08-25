@done
@TUI-108
@ui-refinement
@tui
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

  Scenario: Opening the Changed Files view before the scan returns shows the loading dialog instead of the empty message
    Given the Changed Files view is opened
    When the changed files scan has not yet returned
    Then the body shows the animated loading dialog with the label "Loading changed files…"
    And the body does not show "No changed files"

  Scenario: A completed scan with zero files shows the real empty message
    Given the Changed Files view is opened
    When the changed files scan completes with zero files
    Then the view shows "No changed files"
    And no loading dialog is shown

  Scenario: Selecting a file after the list loads shows the diff stage label until it folds in
    Given the changed files list is loaded with at least one file
    When a file diff load is in flight
    Then the loading dialog shows the label "Loading diff for <file path>…"
    When the diff result folds in
    Then the loading dialog disappears

  Scenario: A stale diff result for a de-selected path does not clear the current stage
    Given the diff stage is in flight for the selected path
    When a diff result arrives for a path that is no longer selected
    Then the current stage's loading state is unchanged

  Scenario: ESC is ignored while the loading dialog is active and closes the view after it flushes
    Given the loading dialog is active
    When the user presses ESC
    Then the view stays open
    When the loading has flushed
    When the user presses ESC
    Then the view emits CloseChangedFilesView

  Scenario: The loading dialog renders through the canonical dialog theme
    Given the loading dialog is active
    When the view is rendered
    Then the dialog shows a rounded border in the cyan accent
    And the dialog title is "Loading changed files"
    And the spinner glyph advances between 0 ms and 80 ms

  Scenario: Arrowing while a diff is in flight is swallowed so the selection stays put
    Given the changed files list is loaded with three files and the first selected
    And the diff for the first file has folded in
    When the user presses Down
    Then the second file is selected and its diff load is in flight
    When the user presses Down again
    Then the key is swallowed and the selection stays on the second file
    When the diff result for the second file arrives
    Then the loading dialog disappears
    And the view shows the diff for the second file
