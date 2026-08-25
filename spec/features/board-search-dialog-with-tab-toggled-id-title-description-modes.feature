@done
@tui
@board
@dialog
@search
@navigation
@BOARD-022
Feature: Board '/' search dialog with Tab-toggled id/title/description modes
  """
  New component rust/fspec-tui/src/components/work_unit_search_dialog.rs (Component, Priority::Foreground, dialog_theme cyan accent, id 'work-unit-search-dialog'), modeled on attachment_picker_dialog.rs. BoardView::handle_event gains a modifier-free Char('/') arm emitting Action::OpenWorkUnitSearch (always consumed). App::dispatch gains handle_open_work_unit_search() (new file app/dispatch_work_unit_search.rs, mirroring dispatch_viewer.rs) which pushes the dialog seeded with the current BoardStore units. Selection emits a new Action::SelectWorkUnit(String) handled in App::dispatch: BoardStore::set_focused_column(status) + a new BoardStore::select_work_unit(id, viewport_height) helper (store/board_viewport.rs) that sets the column selection index and scroll offset so the unit is visible. Dialog owns query + mode enum (Id/Title/Description) + matches (Vec<String> of unit ids) + selected_index + scroll_offset; filtering is a pure fn filter_work_units(&[WorkUnitInfo], mode, query) -> Vec<String> unit-tested with proptest. All files <300 lines.
  RPC architecture: NO new FspecService/FspecBackend RPC methods are introduced. The dialog filters the BoardStore's in-memory snapshot, which is already kept fresh by the existing list_work_units RPC + work_units_rx broadcast (RPC-006). This mirrors the @file popup's transport split: its search_files RPC exists because file data is NOT in the TUI, whereas work units ARE in the TUI. Cross-transport parity is preserved automatically because both embedded and WebSocket transports serve the same work-units snapshot; a source-shape test pins that no new search_work_units RPC appears in rpc/src/lib.rs, transport/mod.rs, embedded.rs, or websocket.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing '/' (modifier-free) on the board opens the work-unit search dialog and consumes the key; the dialog is a Priority::Foreground modal on the Compositor, rendered with the canonical dialog_theme (cyan accent, like the AgentView @file popup)
  #   2. The dialog has three search modes — Id, Title, Description — cycled by pressing Tab (id → title → description → id). The active mode is displayed in the dialog (e.g. in the title row or a mode indicator) and switching modes re-runs the filter with the current query text and resets the selection to the first match
  #   3. Filtering is client-side over the BoardStore's in-memory work units (no new RPC/backend surface): case-insensitive substring match of the query against the active mode's field — Id mode matches work-unit id, Title mode matches title, Description mode matches description (units with no description never match in Description mode). An empty query lists all work units in the active mode's ordering
  #   4. Inside the dialog: printable characters and Backspace edit the query (re-filtering live); Up/Down/PageUp/PageDown/Home/End navigate matches with wrap-around; Enter selects the highlighted match — the board's focused column switches to the match's status column, the row selection moves to the match (viewport-aware so it is visible), the dialog closes; Esc closes the dialog with no selection change; '/' while the dialog is open is ignored (no re-open)
  #   5. When the filter yields zero matches, the dialog shows a single non-selectable '(no work units match "<query>")' row (mirroring the @file popup empty state) and Enter is a no-op; when the board has zero work units, the dialog shows '(board is empty)' and Enter is a no-op
  #   6. The board header keybinding chord gains a '/ Search' segment and the board HelpDialog keybinding list gains a '/' row, so the shortcut is discoverable (parity with the '.' New Agent / 'a' Attachments hints added in RPC-395 / RPC-374)
  #
  # EXAMPLES:
  #   1. Board has AUTH-001 (backlog), RPC-100 (implementing), TUI-110 (done). User presses '/', types 'auth' in Id mode → one row 'AUTH-001'; Enter moves focus to the backlog column on AUTH-001 and closes the dialog
  #   2. User presses '/', types 'search', presses Tab → the mode indicator switches from Id to Title and the same query 'search' is re-matched against titles, so the result list changes to units whose titles contain 'search'
  #   3. A unit with no description never appears in Description-mode results, no matter what the query is
  #   4. The board header chord row displays '/ Search' as a segment, and the board help dialog lists '/' with the description 'Search work units'
  #
  # ========================================
  Background: User Story
    As a Rust TUI board user
    I want to press '/' to open a search dialog that finds a work unit by ID, title, or description, toggling the search mode with Tab
    So that I can jump to any card in a large board without scrolling through all seven columns

  Scenario: Pressing '/' on the board opens the work-unit search dialog
    Given a board with work units "AUTH-001" in backlog and "RPC-100" in implementing
    When I press the '/' key on the board
    Then the work-unit search dialog is open
    And the dialog shows the Id search mode
    And the key event is consumed

  Scenario: Searching by id filters the result list live
    Given a board with work units "AUTH-001" in backlog, "RPC-100" in implementing and "TUI-110" in done
    When I open the search dialog with '/'
    And I type "auth" into the dialog
    Then the dialog lists exactly one match "AUTH-001"

  Scenario: Selecting a match focuses its card on the board and closes the dialog
    Given a board with work units "AUTH-001" in backlog, "RPC-100" in implementing and "TUI-110" in done
    When I open the search dialog with '/'
    And I type "auth" into the dialog
    And I press Enter
    Then the board focuses the backlog column
    And the board selects work unit "AUTH-001"
    And the work-unit search dialog is closed

  Scenario: Pressing Tab toggles the search mode and re-filters with the same query
    Given a board with a work unit "BOARD-022" in backlog whose title contains "search"
    When I open the search dialog with '/'
    And I type "search" into the dialog
    And I press Tab
    Then the dialog shows the Title search mode
    And the dialog lists work unit "BOARD-022" as a match

  Scenario: Pressing Tab cycles through all three modes and returns to Id
    Given the work-unit search dialog is open in Id mode
    When I press Tab twice
    Then the dialog shows the Description search mode
    When I press Tab once more
    Then the dialog shows the Id search mode

  Scenario: Description mode never matches a unit without a description
    Given a board with a work unit "NO-DESC-1" in backlog that has no description
    When I open the search dialog with '/'
    And I switch the search mode to Description
    And I type "anything" into the dialog
    Then the dialog lists no matches

  Scenario: A unit with a matching description appears in Description mode
    Given a board with a work unit "DOC-001" in backlog whose description contains "viewer"
    When I open the search dialog with '/'
    And I switch the search mode to Description
    And I type "viewer" into the dialog
    Then the dialog lists exactly one match "DOC-001"

  Scenario: Pressing Esc closes the dialog without changing the board selection
    Given a board with work unit "AUTH-001" in backlog selected
    When I open the search dialog with '/'
    And I press Esc
    Then the work-unit search dialog is closed
    And the board still selects work unit "AUTH-001"

  Scenario: Pressing '/' while the dialog is open does not re-open it
    Given the work-unit search dialog is open
    When I press the '/' key
    Then the work-unit search dialog is open
    And the dialog is not stacked twice

  Scenario: A query with no matches shows the empty-state row and Enter is a no-op
    Given a board with a single work unit "AUTH-001" in backlog
    When I open the search dialog with '/'
    And I type "zzz-no-such-unit" into the dialog
    Then the dialog shows the empty state for the query "zzz-no-such-unit"
    When I press Enter
    Then the work-unit search dialog is still open

  Scenario: Opening the dialog on an empty board shows the board-is-empty state
    Given a board with no work units
    When I open the search dialog with '/'
    Then the dialog shows the board is empty
    When I press Enter
    Then the work-unit search dialog is still open

  Scenario: The board header chord shows the '/' search shortcut
    Given a board with any selection state
    When the board is rendered
    Then the header chord row contains the segment "/ Search"

  Scenario: The board help dialog lists the '/' search shortcut
    Given the board help dialog is open
    When the help content is inspected
    Then it lists the '/' key with the description "Search work units"
