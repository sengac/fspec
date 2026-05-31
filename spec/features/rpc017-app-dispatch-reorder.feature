@done
@RPC-017
@rust
@tui
@board-view
@work-units
Feature: RPC-017 App dispatch wiring for Action::ReorderUp / ReorderDown
  """
  RPC-017 (slice 3 of 3) — `App::dispatch` no longer drops
  `Action::ReorderUp` / `Action::ReorderDown` to a no-op handler.
  Instead, each variant reads the focused-column selection from
  `BoardStore::selected_work_unit()`, clones the id, and spawns a
  fire-and-forget tokio task that calls
  `backend.move_work_unit_up(id)` or `_down(id)`. The watcher-driven
  `Action::WorkUnitsLoaded` path handles the re-seed after persistence.

  Per RPC-009 single-task tenere: no store mutation happens in the
  spawned task — only the RPC call. The post-write watcher event
  arrives back on the action bus and re-seeds the BoardStore inside
  the same `App::dispatch` flow.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want Action::ReorderUp / Action::ReorderDown to dispatch backend.move_work_unit_up/_down against the focused-column's selected work unit
    So that pressing `[` / `]` in BoardView persists the priority change via the FspecBackend

  Scenario: Action::ReorderUp dispatches backend.move_work_unit_up against the selected work unit
    Given an App constructed against a mock backend whose BoardStore has "B-002" selected in the backlog column
    When the App dispatches Action::ReorderUp
    Then the mock backend records exactly one move_work_unit_up call with id "B-002"
    And the mock backend records zero move_work_unit_down calls

  Scenario: Action::ReorderDown dispatches backend.move_work_unit_down against the selected work unit
    Given an App constructed against a mock backend whose BoardStore has "A-001" selected in the backlog column
    When the App dispatches Action::ReorderDown
    Then the mock backend records exactly one move_work_unit_down call with id "A-001"
    And the mock backend records zero move_work_unit_up calls

  Scenario: Action::ReorderUp is a no-op when the focused column is empty
    Given an App constructed against a mock backend whose BoardStore's focused column is empty
    When the App dispatches Action::ReorderUp
    Then the mock backend records zero move_work_unit_up calls
    And the App does not panic

  Scenario: BoardStore::replace_work_units re-anchors per-column selection to the previously-selected work unit id
    Given a BoardStore seeded with [A-001 backlog, B-002 backlog, C-003 backlog] and the backlog column has selected_index 2 (C-003)
    When store.replace_work_units is called with [A-001 backlog, C-003 backlog, B-002 backlog] (C-003 moved up by one)
    Then store.selected_index_for("backlog") returns 1 (C-003's new position)
    Then store.selected_work_unit().map(|u| u.id.as_str()) returns Some("C-003")
