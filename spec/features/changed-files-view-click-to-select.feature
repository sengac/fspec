@done
@diff-viewer
@tui
@RPC-368
Feature: Click a file row to select it in the Changed Files view

  """
  handle_mouse in views/changed_files/mod.rs gains a MouseEventKind::Down(_) arm placed BEFORE the wheel match. It hit-tests via pane_at(col,row); on the Files pane it computes clicked_index and calls move_selection(clicked_index as i32 - selected_index as i32) to reuse the existing clamp/ensure_visible/Emit(LoadFileDiff) path; on the Diff pane it sets focused_pane=Pane::Diff and returns Consumed.
  last_files_rect is the CONTENT rect (header + underline rows already excluded by pane_header), so no header offset adjustment is needed: row 0 of the content rect maps to file index file_scroll. Clamp clicked_index to files.len()-1 and ignore clicks where (ev.row - rect.y) >= files.len().saturating_sub(file_scroll).
  No App/Navigator changes: Event::Mouse already flows App::handle_event -> Navigator::handle_event -> handle_changed_files_event -> view.handle_event -> handle_mouse, and returning ChangedFilesEvent::Emit is relayed onto action_tx by navigator_events.rs. Test via ChangedFilesView unit tests constructing the view, caching rects through a render pass (or seeding last_files_rect), and dispatching a synthetic crossterm MouseEvent with kind Down(Left).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A left mouse Down on a file row in the file-list pane selects that file (sets selected_index to the clicked row) and requests a diff reload for it
  #   2. The clicked row maps to a file index via clicked_index = file_scroll + (ev.row - last_files_rect.y), accounting for the current scroll offset
  #   3. Clicking a row whose computed index is already the selected file is a no-op (no diff reload emitted)
  #   4. A click in the Files pane focuses the Files pane; a click in the Diff pane focuses the Diff pane without changing the file selection
  #   5. A click that lands below the last file row (empty space) or outside both pane rects does not change the selection
  #
  # EXAMPLES:
  #   1. Listing a.txt then b.txt with a.txt selected, a left click on b.txt's row sets the selected index to 1 and emits Action::LoadFileDiff for b.txt
  #   2. With the file list scrolled so the first visible row is index 3, a click on the top visible row selects file index 3
  #   3. Clicking the already-selected file row emits no diff reload and leaves the selection unchanged
  #   4. Clicking inside the diff pane focuses the diff pane and leaves the file selection unchanged
  #
  # ========================================

  Background: User Story
    As a fspec user browsing the Changed Files view
    I want to click a file row to select it
    So that I can pick a file to diff with the mouse instead of only arrow keys or the wheel

  Scenario: Clicking an unselected file row selects it and reloads its diff
    Given a Changed Files view listing a.txt then b.txt with a.txt selected
    When the user left-clicks the file row for b.txt
    Then the selected index becomes 1
    And the view requests a diff reload for b.txt
    And the focused pane is the file list pane

  Scenario: Clicking the top visible row selects the file at the scroll offset
    Given a Changed Files view whose file list is scrolled so the first visible row is index 3
    When the user left-clicks the top visible file row
    Then the selected index becomes 3
    And the view requests a diff reload for the file at index 3

  Scenario: Clicking the already-selected file row changes nothing
    Given a Changed Files view listing a.txt then b.txt with a.txt selected
    When the user left-clicks the file row for a.txt
    Then the selected index is still 0
    And the view does not request a diff reload

  Scenario: Clicking inside the diff pane focuses it without changing the selection
    Given a Changed Files view listing a.txt then b.txt with a.txt selected
    When the user left-clicks inside the diff pane
    Then the focused pane is the diff pane
    And the selected index is still 0
    And the view does not request a diff reload

  Scenario: Clicking empty space below the last file changes nothing
    Given a Changed Files view listing a.txt then b.txt with a.txt selected
    When the user left-clicks the empty area below the last file row
    Then the selected index is still 0
    And the view does not request a diff reload
