@done
@navigation
@tui
@RPC-023
Feature: BoardView mouse handling — wheel scroll + click focus
  """
  Decision (Q7): Horizontal wheel (ScrollLeft/ScrollRight) inside content area emits FocusPrev/NextColumn (Rust-port-only opportunity vs TS).
  Decision (Q8): Click-to-focus is INCLUDED in this card — adds Action::SetFocusedColumn(usize) and Action::SelectIndexInFocused(usize) variants; BoardStore gains select_index_in_focused().
  BoardView gains last_content_area: Cell<Option<Rect>>, last_column_header_areas: Cell<Option<[Rect; 7]>>, last_column_content_areas: Cell<Option<[Rect; 7]>> — populated at render_with_store, read by handle_event.
  """

  Background: User Story
    As a user of the Rust fspec TUI
    I want to scroll BoardView columns with my mouse wheel, click columns to focus them, and click rows to select work units
    So that I do not need to use the keyboard for every navigation step on the Kanban board

  Scenario: Wheel-down inside the BACKLOG content area emits SelectNext
    Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    And the focused column is BACKLOG and selected_index is 0
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollDown) arrives with the cursor inside the column-content area
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::SelectNext is emitted onto the action bus
    And dispatching that action through App::dispatch advances BoardStore.selected_index_for("backlog") to 1

  Scenario: Wheel-down at the last unit wraps the selection to index 0
    Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    And the focused column is BACKLOG and selected_index is 19
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollDown) arrives with the cursor inside the column-content area
    And the resulting Action::SelectNext is dispatched through App::dispatch
    Then BoardStore.selected_index_for("backlog") wraps back to 0

  Scenario: Wheel-up inside the BACKLOG content area emits SelectPrev
    Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    And the focused column is BACKLOG and selected_index is 5
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollUp) arrives with the cursor inside the column-content area
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::SelectPrev is emitted onto the action bus

  Scenario: Wheel event outside the content area is Ignored
    Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollDown) arrives at row 0 which lies on the top border
    Then BoardView::handle_event returns EventResult::Ignored
    And no Action is emitted onto the action bus

  Scenario: Wheel-right inside the content area emits FocusNextColumn
    Given the BoardStore is seeded with work units across columns
    And the focused column is BACKLOG
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollRight) arrives with the cursor inside the column-content area
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::FocusNextColumn is emitted onto the action bus

  Scenario: Wheel-left inside the content area emits FocusPrevColumn
    Given the BoardStore is seeded with work units across columns
    And the focused column is IMPLEMENTING
    And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    When an Event::Mouse(ScrollLeft) arrives with the cursor inside the column-content area
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::FocusPrevColumn is emitted onto the action bus

  Scenario: Left-click on a column header emits SetFocusedColumn
    Given the BoardStore is seeded with work units across columns
    And the focused column is BACKLOG
    And BoardView has been rendered onto a 120x30 TestBackend so last_column_header_areas is populated
    When an Event::Mouse(Down(Left)) arrives with the cursor inside the SPECIFYING column header rect
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::SetFocusedColumn(1) is emitted onto the action bus
    And dispatching that action through App::dispatch sets BoardStore.focused_column_index() to 1

  Scenario: Left-click on a content row emits SetFocusedColumn and SelectIndexInFocused
    Given the BoardStore is seeded with five story work units in the DONE column
    And the focused column is BACKLOG and DONE has scroll_offset 0
    And BoardView has been rendered onto a 120x30 TestBackend so last_column_content_areas is populated
    When an Event::Mouse(Down(Left)) arrives with the cursor on visible row 2 of the DONE column content area
    Then BoardView::handle_event returns EventResult::Consumed
    And Action::SetFocusedColumn(5) is emitted onto the action bus
    And Action::SelectIndexInFocused(2) is emitted onto the action bus
    And dispatching those actions through App::dispatch leaves BoardStore.focused_column_index() at 5 and BoardStore.selected_index_for("done") at 2

  Scenario: Click on a content row adds scroll_offset to the clicked row index
    Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    And the BACKLOG scroll_offset is 5
    And BoardView has been rendered onto a 120x30 TestBackend so last_column_content_areas is populated
    When an Event::Mouse(Down(Left)) arrives with the cursor on visible row 1 of the BACKLOG column content area
    Then Action::SelectIndexInFocused(6) is emitted onto the action bus
