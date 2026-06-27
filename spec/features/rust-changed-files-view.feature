@done
@diff-viewer
@tui
@RPC-356
Feature: Dual-pane ChangedFilesView with F-key board wiring and Navigator integration

  """
  ChangedFilesView lives in views/changed_files/ split into mod.rs (state + event handling), render.rs (panes), row.rs (file-row formatting), diff_render.rs (colored diff lines), each under 300 lines.
  Wiring follows the Navigator/Action pattern: BoardView::handle_event emits Action::OpenChangedFilesView; Navigator gains ViewMode::ChangedFiles + owned ChangedFilesView; App::dispatch loads data via backend.changed_files()/file_diff() mirroring the checkpoint_counts -> CheckpointCountsLoaded flow. Reuses scroll_viewport WheelVelocity + ensure_visible for scroll math.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing F (or f) on the board opens the Changed Files view via Action::OpenChangedFilesView and the Navigator flips to ViewMode::ChangedFiles
  #   2. The file list shows each changed file with a colored A/M/D/R status letter (A=green, M=yellow, D=red, R=cyan) and the path
  #   3. The currently selected file row shows a > cursor; other rows show a space
  #   4. The diff pane shows the unified diff of the selected file with + add lines green, - remove lines red, and @@ hunk headers dim/cyan
  #   5. Moving the selection with Up/Down reloads the diff pane for the newly selected file
  #   6. Tab (and Left/Right) toggles focus between the file list pane and the diff pane
  #   7. PgUp/PgDn and the mouse wheel scroll the focused pane using the shared WheelVelocity ramp
  #   8. Esc returns to the board
  #   9. With no changed files, the view shows a No changed files empty-state message and Esc still returns to the board
  #
  # EXAMPLES:
  #   1. Pressing F on the board emits Action::OpenChangedFilesView and consumes the event
  #   2. Applying Action::OpenChangedFilesView flips the Navigator to ViewMode::ChangedFiles; CloseChangedFilesView flips it back to Board
  #   3. A view with M a.txt and A b.txt renders a yellow M then a green A, with the > cursor on the selected row
  #   4. The diff pane renders +added lines in green, -removed lines in red, and @@ hunk headers dim
  #   5. Pressing Down from a.txt to b.txt moves the selection to index 1 and emits a diff reload for b.txt
  #   6. Pressing Tab moves focus to the diff pane so PgDn scrolls the diff and not the file list
  #   7. With no changed files the view shows No changed files and Esc emits Action::CloseChangedFilesView
  #   8. A mouse wheel ScrollDown over the focused diff pane scrolls it by the WheelVelocity step
  #
  # ========================================

  Background: User Story
    As a fspec user on the Kanban board
    I want to open a dual-pane Changed Files view from the board and browse file diffs
    So that I can review my working-tree changes without leaving the TUI

  Scenario: Pressing F on the board opens the Changed Files view
    Given the BoardView is the active view
    When the user presses the F key
    Then the BoardView emits Action::OpenChangedFilesView
    And the key event is consumed

  Scenario: Opening flips the Navigator to the Changed Files view and closing returns to the board
    Given the Navigator is showing the Board view
    When Action::OpenChangedFilesView is applied to the Navigator
    Then the Navigator active view is ViewMode::ChangedFiles
    When Action::CloseChangedFilesView is applied to the Navigator
    Then the Navigator active view is ViewMode::Board

  Scenario: The file list renders colored status letters and a selection cursor
    Given a Changed Files view with a modified a.txt and an added b.txt
    When the view is rendered
    Then the row for a.txt shows a yellow M status letter
    And the row for b.txt shows a green A status letter
    And the selected row shows a > cursor while other rows show a space

  Scenario: The diff pane renders colored add, remove and hunk-header lines
    Given a Changed Files view whose selected file diff has an added, a removed and a hunk-header line
    When the view is rendered
    Then the diff pane shows the added line in green
    And the diff pane shows the removed line in red
    And the diff pane shows the hunk-header line dimmed

  Scenario: Moving the selection down reloads the diff for the newly selected file
    Given a Changed Files view listing a.txt then b.txt with a.txt selected
    When the user presses the Down key
    Then the selected index becomes 1
    And the view requests a diff reload for b.txt

  Scenario: Tab moves focus to the diff pane so PgDn scrolls the diff
    Given a Changed Files view focused on the file list pane
    When the user presses the Tab key
    Then the focused pane is the diff pane
    When the user presses the PgDn key
    Then the diff pane scroll offset increases and the file selection is unchanged

  Scenario: An empty repo shows the empty-state message and Esc returns to the board
    Given a Changed Files view with no changed files
    When the view is rendered
    Then the view shows the No changed files message
    When the user presses the Esc key
    Then the view emits Action::CloseChangedFilesView

  Scenario: A mouse wheel scroll over the focused diff pane scrolls it by the WheelVelocity step
    Given a Changed Files view focused on the diff pane with a long diff
    When a mouse wheel ScrollDown event arrives over the diff pane
    Then the diff pane scroll offset advances by the WheelVelocity step

  Scenario: Diff pane scroll stops at the last full page
    Given a Changed Files view focused on the diff pane with a long diff and a known pane height
    When the user pages down far past the end of the diff
    Then the diff pane scroll offset never exceeds the diff line count minus the pane height
