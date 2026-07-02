@done
@diff-viewer
@tui
@RPC-364
Feature: Three-pane CheckpointsView with C-key board wiring and Navigator integration
  """
  Greenfield views/checkpoints/ module modeled on views/changed_files/ but with THREE panes (Checkpoints list, Files list, Diff) and a focus state machine (Checkpoints->Files->Diff). Board C/c key emits Action::OpenCheckpointsView; Navigator owns CheckpointsView via ViewMode::Checkpoints. App::dispatch_checkpoints spawns lazy loads: list_checkpoints -> CheckpointsLoaded; checkpoint_diff_files -> CheckpointFilesLoaded; checkpoint_file_diff -> CheckpointFileDiffLoaded (stale results dropped by key). Reuses RPC-363 shared diff_common helpers (diff_line, file_row, render_pane_scrollbar) and components/scroll_viewport (WheelVelocity, ensure_visible) + list_scrollbar. Browse + diff only; restore/delete are RPC-365/366. Auto checkpoints render as '{workUnitId}: {Phase}', manual show raw name; list most-recent-first capped 200. Esc returns to board.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing C (or c) on the board opens the Checkpoints view via Action::OpenCheckpointsView and the Navigator flips to ViewMode::Checkpoints
  #   2. The view has three panes (Checkpoints list, Files list, Diff) and the focused pane's heading is highlighted
  #   3. Tab and Right cycle focus forward Checkpoints->Files->Diff->Checkpoints; Left cycles backward
  #   4. Automatic checkpoints render as '{workUnitId}: {Phase}' and manual checkpoints render their raw name; the list is sorted most-recent-first
  #   5. Selecting a checkpoint loads its changed files; selecting a file loads and shows its colored unified diff
  #   6. Arrow keys act on the focused pane: list panes move selection, the diff pane scrolls; scrollbars appear only when a pane's content overflows
  #   7. With no checkpoints the view shows a 'No checkpoints available' message and Esc returns to the board
  #
  # EXAMPLES:
  #   1. Pressing C on the board emits Action::OpenCheckpointsView and consumes the event
  #   2. Applying Action::OpenCheckpointsView flips the Navigator to ViewMode::Checkpoints; CloseCheckpointsView flips it back to Board
  #   3. A list with an auto checkpoint AUTH-001-auto-testing and a manual 'baseline' renders 'AUTH-001: Testing' and 'baseline'
  #   4. Pressing Tab from the Checkpoints pane moves focus to the Files pane and highlights its heading
  #   5. Selecting a checkpoint that changed a.txt then selecting a.txt shows a.txt's colored diff in the diff pane
  #   6. With the Diff pane focused, pressing Down scrolls the diff one line; with the Checkpoints pane focused, Down moves the checkpoint selection
  #   7. With no checkpoints the view shows 'No checkpoints available' and pressing Esc emits Action::CloseCheckpointsView
  #
  # ========================================
  Background: User Story
    As a fspec user on the Kanban board
    I want to press C to open a three-pane Checkpoints viewer and browse checkpoints, their files and diffs
    So that I can review my saved checkpoints without leaving the TUI

  Scenario: Pressing C on the board opens the Checkpoints view
    Given the Kanban board is focused
    When the user presses the C key
    Then the board emits Action::OpenCheckpointsView
    And the key event is consumed

  Scenario: Opening flips the Navigator to the Checkpoints view and closing returns to the board
    Given a Navigator whose active view is the Board
    When Action::OpenCheckpointsView is applied
    Then the Navigator active view is Checkpoints
    When Action::CloseCheckpointsView is applied
    Then the Navigator active view is Board

  Scenario: Automatic checkpoints render as id and phase while manual checkpoints render their raw name
    Given a Checkpoints view listing an automatic checkpoint AUTH-001-auto-testing and a manual checkpoint baseline
    When the view is rendered
    Then the checkpoints pane shows the row AUTH-001: Testing
    And the checkpoints pane shows the row baseline

  Scenario: Tab moves focus from the Checkpoints pane to the Files pane and highlights its heading
    Given a Checkpoints view focused on the Checkpoints pane
    When the user presses the Tab key
    Then the focused pane is the Files pane
    And the Files pane heading is highlighted

  Scenario: Selecting a checkpoint loads its files and selecting a file shows its colored diff
    Given a Checkpoints view whose selected checkpoint changed a.txt
    When the checkpoint files for a.txt are loaded
    And the unified diff for a.txt is loaded
    When the view is rendered
    Then the diff pane shows the added line in green

  Scenario: Arrow keys act on the focused pane
    Given a Checkpoints view with the Diff pane focused and a long diff
    When the user presses the Down key
    Then the diff pane scroll offset increases and the checkpoint selection is unchanged
    Given a Checkpoints view with the Checkpoints pane focused and two checkpoints
    When the user presses the Down key
    Then the selected checkpoint index becomes 1

  Scenario: Empty repo shows a no-checkpoints message and Esc returns to the board
    Given a Checkpoints view with no checkpoints
    When the view is rendered
    Then the view shows the No checkpoints available message
    When the user presses the Esc key
    Then the view emits Action::CloseCheckpointsView
