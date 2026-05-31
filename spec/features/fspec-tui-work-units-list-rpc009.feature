@done
@tui
@rust
@infrastructure
@parity
@rpc
@RPC-009
@critical
Feature: Work units list view (RPC-009)
  """
  Renders via ratatui core `List` + `ListState` + `Block::default().borders(Borders::ALL)` (NOT tui-widget-list — that's RPC-002 Slice 03/04). State: `WorkUnitsListView { items: Vec<WorkUnitInfo>, state: ListState, focused: bool }`. Each `ListItem` is `format!("{} {}", id, status)` styled by status from the existing Theme. Selection is single-item via `state.select(Some(idx))`. j/Down → `state.select(Some((i+1).min(items.len().saturating_sub(1))))`; k/Up → `state.select(Some(i.saturating_sub(1)))`. Focused border style swaps on the wrapping Block based on `focused: bool`. Reads `backend.work_units_rx()` for live updates (via App-level subscriber task converting messages into Action::WorkUnitsLoaded) and is seeded by `backend.list_work_units()` during App bootstrap.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want WorkUnitsListView to render a flat single-selection list of work units (id + status) via ratatui core List + ListState, accept j/k or Up/Down navigation only when focused, replace its in-memory list when Action::WorkUnitsLoaded arrives, and display the focused/unfocused border style on its surrounding Block
    So that the LEFT pane of the basic frontend reads `backend.work_units_rx()` and shows live work-units state without virtualisation or a virtualised list crate

  Scenario: WorkUnitsListView seeds its list from the bootstrap snapshot and selects index 0
    Given a WorkUnitsListView constructed against a focused: true initial state and an 80x24 TestBackend
    And a Vec<WorkUnitInfo> with two entries [AUTH-001 done, AUTH-002 implementing]
    When the App's bootstrap dispatches `Action::WorkUnitsLoaded` carrying the seed list
    Then the WorkUnitsListView's items field equals the seed list
    And the WorkUnitsListView's state.selected() returns Some(0)
    And the rendered buffer contains a row with the substring "AUTH-001 done"
    And the rendered buffer contains a row with the substring "AUTH-002 implementing"

  Scenario: Action::WorkUnitsLoaded replaces the in-memory list (live update)
    Given a WorkUnitsListView already seeded with [AUTH-001 done, AUTH-002 implementing]
    When `Action::WorkUnitsLoaded(vec![AUTH-001 done, AUTH-002 implementing, AUTH-003 backlog])` arrives via `update`
    Then the WorkUnitsListView's items field equals the three-entry list
    And the rendered buffer contains a row with the substring "AUTH-003 backlog"

  Scenario: WorkUnitsLoaded clamps the selection when the new list is shorter
    Given a WorkUnitsListView seeded with three entries and `state.select(Some(2))`
    When `Action::WorkUnitsLoaded(vec![AUTH-001 done, AUTH-002 implementing])` shrinks the list to two entries
    Then the WorkUnitsListView's state.selected() returns Some(1)

  Scenario: j/Down moves the selection forward and clamps at the last index
    Given a focused WorkUnitsListView seeded with three entries and selection at index 0
    When the view processes a synthetic Key('j') event
    Then state.selected() returns Some(1)
    When the view processes a synthetic Key(Down) event
    Then state.selected() returns Some(2)
    When the view processes another synthetic Key('j') event
    Then state.selected() returns Some(2)

  Scenario: k/Up moves the selection backward and clamps at index 0
    Given a focused WorkUnitsListView seeded with three entries and selection at index 2
    When the view processes a synthetic Key('k') event
    Then state.selected() returns Some(1)
    When the view processes a synthetic Key(Up) event
    Then state.selected() returns Some(0)
    When the view processes another synthetic Key('k') event
    Then state.selected() returns Some(0)

  Scenario: Navigation keys are ignored when the view is not focused
    Given a WorkUnitsListView with `focused = false` seeded with three entries and selection at index 0
    When the view processes a synthetic Key('j') event
    Then handle_event returns `EventResult::Ignored(None)`
    And state.selected() still returns Some(0)

  Scenario: Focused and unfocused borders use different styles from the Theme
    Given a WorkUnitsListView with `focused = true` rendered onto an 80x24 TestBackend
    And a sibling WorkUnitsListView with `focused = false` rendered onto an 80x24 TestBackend
    When the rendered top-left corner cell of each surrounding Block is inspected
    Then the focused buffer's top-left cell carries the Theme's `border_focused` style
    And the unfocused buffer's top-left cell carries the Theme's `border` style

  Scenario: Each list item is rendered as "{id} {status}"
    Given a WorkUnitsListView seeded with [AUTH-001 done, AUTH-002 implementing]
    When the view is rendered onto an 80x24 TestBackend
    Then the buffer contains exactly the substring "AUTH-001 done"
    And the buffer contains exactly the substring "AUTH-002 implementing"

  Scenario: Enter on the work-units list does nothing visible (selection-only navigation)
    Given a focused WorkUnitsListView with selection at index 1
    When the view processes a synthetic Key(Enter) event
    Then handle_event returns `EventResult::Ignored(None)`
    And state.selected() still returns Some(1)
    And the App's `should_quit` flag is unchanged
