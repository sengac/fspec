@done
@RPC-016
@rust
@tui
@ui
@rpc
@ui-enhancement
@board-view
@viewport
@kanban
@scroll
@indicators
Feature: RPC-016 BoardView per-column scroll viewport + ⏩/🟢 indicators + keyboard navigation
  """
  RPC-016 (slice 1 of 3) — BoardView gains the per-column scroll viewport
  semantics from src/tui/components/UnifiedBoardLayout.tsx. Each of the
  seven kanban columns owns its own `scroll_offset` and renders at most
  `viewport_height` rows of work units.

  - When `scroll_offset > 0`, the first viewport row of that column
  renders `↑` centered.
  - When `scroll_offset + viewport_height < units.len()`, the last
  viewport row of that column renders `↓` centered.
  - The most-recently-changed work unit (derived from the new
  `WorkUnitInfo.last_state_change_at` ISO-8601 field) renders as
  `⏩ {session_indicator}{id}{points} ⏩` in every column it appears.
  - Work units with an entry in `BoardStore.session_attachments`
  render with a `🟢 ` prefix before the id.
  - Moving the selection past the visible viewport auto-scrolls the
  focused column so the selection stays visible (accounting for the
  ↑/↓ arrow rows each consuming one viewport row).
  - `PageUp`/`PageDown` scroll the focused column's selection by
  `viewport_height` rows. `Home`/`End` jump the focused column's
  selection to the first / last unit.

  No new RPC methods land in this card — the session-attached indicator
  reuses the existing `BoardStore::session_attachments` map already wired
  in RPC-012, and the last-changed indicator reuses the extended
  `WorkUnitInfo` payload now carrying `last_state_change_at`.

  No TypeScript source files in `src/tui/` are modified — the TS
  BoardView's `lastChangedWorkUnit` derivation continues to use
  `stateHistory[last].timestamp` because `last_state_change_at` is
  purely additive.

  Pair: render tests live in
  codelet/fspec-tui/tests/view_board_unit_rpc016.rs; viewport math /
  store tests live in codelet/fspec-tui/tests/store_board_viewport_rpc016.rs;
  source-shape regressions live in
  codelet/fspec-tui/tests/source_shape_rpc016.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want the BoardView to render a scroll-aware per-column viewport with ⏩/🟢 indicators, ↑/↓ arrows, and PageUp/PageDown/Home/End keyboard navigation
    So that the Rust ratatui BoardView matches the TS Ink UnifiedBoardLayout per-column scroll behaviour and visually surfaces the most-recently-changed and session-attached work units

  Scenario: Column with no scroll renders the down arrow on the last viewport row
    Given a BoardStore seeded with twenty story work units all in the backlog column
    And the focused column is "backlog" and the selected index is 0
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column contain the glyph "↓" on the last viewport row
    And the column-content rows for the BACKLOG column do NOT contain the glyph "↑" on the first viewport row

  Scenario: Column with mid-range scroll renders both up and down arrows
    Given a BoardStore seeded with twenty story work units all in the backlog column
    And the BACKLOG scroll_offset is 5
    And the focused column is "backlog"
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column contain the glyph "↑" on the first viewport row
    And the column-content rows for the BACKLOG column contain the glyph "↓" on the last viewport row

  Scenario: Column with fewer units than viewport_height renders no arrows
    Given a BoardStore seeded with three story work units all in the backlog column
    And the focused column is "backlog"
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column do NOT contain the glyph "↑"
    And the column-content rows for the BACKLOG column do NOT contain the glyph "↓"

  Scenario: Most-recently-changed work unit renders the ⏩ ⏩ prefix and suffix
    Given a BoardStore seeded with AUTH-001 last_state_change_at "2026-05-13T10:00:00Z" and AUTH-002 last_state_change_at "2026-05-14T10:00:00Z" in the backlog column
    And the focused column is "specifying" so neither unit is the selected highlighted cell
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column contain the substring "⏩ AUTH-002"
    And the column-content rows for the BACKLOG column contain the substring "AUTH-002 ⏩"
    And the column-content rows for the BACKLOG column do NOT contain the substring "⏩ AUTH-001"

  Scenario: Work unit with an attached session renders the 🟢 prefix
    Given a BoardStore seeded with AUTH-002 (story, backlog, estimate 5) and AUTH-001 (story, backlog) with last_state_change_at on AUTH-001 strictly greater than AUTH-002
    And the BoardStore has an attached session for AUTH-002
    And the focused column is "specifying" so neither unit is the selected highlighted cell
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column contain the substring "🟢 AUTH-002 [5]"

  Scenario: Last-changed and session-attached indicators stack on the same unit
    Given a BoardStore seeded with AUTH-002 (story, backlog) carrying the largest last_state_change_at
    And the BoardStore has an attached session for AUTH-002
    And the focused column is "specifying" so AUTH-002 is not the selected highlighted cell
    When the App renders BoardView against a 120x24 TestBackend
    Then the column-content rows for the BACKLOG column contain the substring "⏩ 🟢 AUTH-002"
    And the column-content rows for the BACKLOG column contain the substring "AUTH-002 ⏩"

  Scenario: PageDown advances the focused column's selection by viewport_height rows
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 0
    When BoardView handles a PageDown key event against the store
    Then the action bus carries an Action::ScrollFocusedColumnDown variant whose payload equals the most recent viewport_height observed by BoardView

  Scenario: PageUp scrolls the focused column's selection back by viewport_height rows
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 25
    When BoardView handles a PageUp key event against the store
    Then the action bus carries an Action::ScrollFocusedColumnUp variant whose payload equals the most recent viewport_height observed by BoardView

  Scenario: Home jumps the focused column's selection to the first unit
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 20
    When BoardView handles a Home key event against the store
    Then the action bus carries the Action::SelectFirstInFocused variant

  Scenario: End jumps the focused column's selection to the last unit
    Given a BoardStore seeded with thirty story work units all in the backlog column
    And the focused column is "backlog" with selected index 0
    When BoardView handles an End key event against the store
    Then the action bus carries the Action::SelectLastInFocused variant

  Scenario: RPC-014 details strip and RPC-015 header are still painted after RPC-016 lands
    Given a BoardStore containing AUTH-001 (story, backlog, title "User Login", description "Sign in with email/password", estimate 5, epic "authentication", no attachments)
    And the focused column is "backlog" and the selected index is 0
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "AUTH-001: User Login"
    And the rendered buffer contains the substring "Epic: authentication"
    And the rendered buffer contains the substring "Status: backlog"
    And the rendered buffer contains the substring "Checkpoints: None"
    And the rendered buffer contains the substring "← →"
    And the rendered buffer contains the substring "Work Agent"
