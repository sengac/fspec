@done
@RPC-016
@rust
@tui
@store
@viewport
@board-view
@scroll
@kanban
Feature: RPC-016 BoardStore scroll_offsets + viewport math
  """
  RPC-016 (slice 2 of 3) — BoardStore gains per-column `scroll_offsets`
  plus four pure mutation methods that the App task calls inside
  `App::dispatch`:

  - `scroll_offset_for(column) -> usize`
  - `set_scroll_offset_for(column, offset)`
  - `move_selection(delta, viewport_height)` — clamps selection and
  auto-scrolls the focused column.
  - `scroll_focused_column(delta, viewport_height)` — PageUp/PageDown.
  - `select_first_in_focused()` / `select_last_in_focused()` —
  Home/End.

  All mutations remain on the App task per the RPC-009 single-task
  invariant. No Mutex / RwLock / atomics anywhere on the store surface.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want pure BoardStore methods for scroll offsets and viewport-aware selection
    So that App::dispatch can mutate the per-column scroll state synchronously without spawning tasks

  Scenario: Default BoardStore reports zero scroll_offset for every column
    Given a freshly constructed BoardStore via BoardStore::default()
    When the developer reads scroll_offset_for for each of the seven canonical columns
    Then every column returns 0

  Scenario: set_scroll_offset_for stores per-column offsets independently
    Given a freshly constructed BoardStore via BoardStore::default()
    When the developer calls set_scroll_offset_for("backlog", 4)
    And the developer calls set_scroll_offset_for("done", 12)
    Then scroll_offset_for("backlog") returns 4
    And scroll_offset_for("done") returns 12
    And scroll_offset_for("implementing") returns 0

  Scenario: move_selection within visible viewport leaves scroll_offset unchanged
    Given a BoardStore seeded with twenty story work units all in the backlog column
    And the focused column is "backlog" with selected index 3 and scroll_offset 0
    When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    Then selected_index_for("backlog") returns 4
    And scroll_offset_for("backlog") returns 0

  Scenario: move_selection beyond bottom of viewport scrolls the focused column down
    Given a BoardStore seeded with twenty story work units all in the backlog column
    And the focused column is "backlog" with selected index 9 and scroll_offset 0
    When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    Then selected_index_for("backlog") returns 10
    And scroll_offset_for("backlog") is strictly greater than 0
    And the selected index remains inside the visible viewport window

  Scenario: move_selection above top of viewport scrolls the focused column up
    Given a BoardStore seeded with twenty story work units all in the backlog column
    And the focused column is "backlog" with selected index 5 and scroll_offset 5
    When the developer calls move_selection(-1, 10) (move up by 1 with viewport_height 10)
    Then selected_index_for("backlog") returns 4
    And scroll_offset_for("backlog") is strictly less than 5

  Scenario: scroll_focused_column advances the selection by viewport_height
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 0 and scroll_offset 0
    When the developer calls scroll_focused_column(1, 10)
    Then selected_index_for("backlog") returns 10
    And scroll_offset_for("backlog") is strictly greater than 0

  Scenario: select_first_in_focused resets the focused column to index 0 with offset 0
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 17 and scroll_offset 8
    When the developer calls select_first_in_focused()
    Then selected_index_for("backlog") returns 0
    And scroll_offset_for("backlog") returns 0

  Scenario: select_last_in_focused jumps to the last unit
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 0 and scroll_offset 0
    When the developer calls select_last_in_focused()
    Then selected_index_for("backlog") returns 29

  Scenario: move_selection wraps to first index when moving past the last unit
    Given a BoardStore seeded with three story work units all in the backlog column
    And the focused column is "backlog" with selected index 2 and scroll_offset 0
    When the developer calls move_selection(1, 10) (move down by 1 with viewport_height 10)
    Then selected_index_for("backlog") returns 0
    And scroll_offset_for("backlog") returns 0

  Scenario: move_selection wraps to last index when moving above the first unit
    Given a BoardStore seeded with five story work units all in the backlog column
    And the focused column is "backlog" with selected index 0 and scroll_offset 0
    When the developer calls move_selection(-1, 10) (move up by 1 with viewport_height 10)
    Then selected_index_for("backlog") returns 4
    And the visible viewport contains the selected index
