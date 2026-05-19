@agent-view
@tui-component
@done
@scrolling
@navigation
@tui
@RPC-028
Feature: Add proper scrolling, mouse wheel support, and wrap-around to all Rust dialog/popup/picker views

  """
  [0] Introduce codelet/fspec-tui/src/components/scroll_viewport.rs with three primitives: wrap_index(current, delta, total) using rem_euclid (mirrors store/board_viewport.rs:42-44), ensure_visible(scroll_offset, selected, visible_rows, total) (popup-flavour of board_viewport::adjust_scroll_offset without the two-pass arrow correction), and WheelVelocity with last/vel Cells and a step(direction) method that mirrors TS AgentView.tsx:4435-4458.
  [1] Each migrated view adds scroll_offset: usize and last_visible_rows: Cell<usize> fields, replaces iter().take(10) with iter().skip(scroll_offset).take(visible_rows), and wires Up/Down/PgUp/PgDn/Home/End + ScrollUp/ScrollDown through wrap_index + ensure_visible. The dialog frame in components/dialog_theme.rs already provides the body height — exposing it via dialog_rect is enough; visible_rows = body.height - title - gap - footer.
  [2] Mouse events route through the existing Compositor (no parallel pipeline). Each view exposes handle_mouse(&mut self, ev: MouseEvent, rect: Rect, vr: usize) called by AgentView's mouse branch after hit-testing the popup_rect. Events that fall outside the popup_rect return EventResult::Ignored so they bubble to AgentView's scrollback/board. The TUI-078 MouseTrackingToggle (5-second tokio re-enable timer) remains the responsibility of the App shell — popups do not own it.
  [3] Migration order (smallest blast radius first): (a) scroll_viewport.rs + unit tests; (b) SlashCommandPopup (the reported defect); (c) FileSearchPopup (copy-paste-adapt); (d) ModelSelectorDialog (windowing + Pg keys + mouse, snapshot regen); (e) ResumeSessionView (replace bespoke wrap math with wrap_index + ensure_visible, add mouse + Pg keys); (f) SearchHistoryView (same as e); (g) ThinkingLevelDialog (mouse-wheel only); (h) regenerate every touched insta snapshot.
  [4] TS Ink parity invariants (DO NOT modify TS sources): wrap-around behaviour matches src/tui/hooks/useSlashCommandInput.ts:156-174 + useFileSearchInput.ts:167-185 + ThinkingLevelDialog.tsx:98-110; mouse-wheel acceleration matches AgentView.tsx:4435-4458 (1×–5× ramp at <150ms intervals). The Rust port goes beyond the TS source by adding mouse-wheel to popups (TS only wires it for the Resume picker) — this is mandated by RPC-002 + RPC-023 master plan.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SlashCommandPopup, FileSearchPopup, ModelSelectorDialog, ResumeSessionView, and SearchHistoryView MUST maintain a scroll_offset and window the visible rows via items[scroll_offset..scroll_offset+visible_rows] — the `.take(10)` hard cap in slash/file popups is removed.
  #   2. Up/Down navigation in every selectable view wraps around the ends using rem_euclid (matching BoardStore::move_selection) and always calls adjust_scroll afterward so the post-wrap selection is visible.
  #   3. Every selectable view handles MouseEventKind::ScrollUp/ScrollDown by emitting move_by(±1) and hit-tests the event against its last-rendered Rect (events outside the rect are Ignored).
  #   4. PageUp/PageDown jump by exactly visible_rows; Home selects index 0 with scroll_offset 0; End selects len-1 with scroll_offset adjusted so the last row is visible.
  #   5. ↑ glyph is painted on the top body row when scroll_offset > 0; ↓ glyph on the bottom body row when scroll_offset + visible_rows < total — same idiom as views/board/viewport.rs:95-101.
  #   6. A new shared module codelet/fspec-tui/src/components/scroll_viewport.rs owns wrap_index(current, delta, total), ensure_visible(scroll_offset, selected, visible_rows, total), and a WheelVelocity helper — every refactored view consumes it instead of duplicating math.
  #   7. Mouse-wheel acceleration follows TS AgentView.tsx:4435-4458 — ≥5 wheel events within 150 ms cap velocity at 5; gap ≥150 ms resets to 1 — and is shared via WheelVelocity in scroll_viewport.rs.
  #   8. No refactored view file exceeds 300 LoC; helpers move into scroll_viewport.rs (or per-view _rows.rs siblings) as needed.
  #
  # EXAMPLES:
  #   1. User opens SlashCommandPopup with 14 matching commands, presses Down 10 times: the 11th match is selected AND visible — scroll_offset advanced to 4 (visible_rows=10), and ↑ glyph appears on the top body row.
  #   2. User in SlashCommandPopup at the last match presses Down: selection wraps to the first match and the popup re-scrolls to show row 0.
  #   3. User scrolls the trackpad wheel up while hovering inside FileSearchPopup: the selection moves up one row; if at the first row, it wraps to the last match.
  #   4. User in ModelSelectorDialog with 30 models presses PageDown: selection jumps forward by visible_rows skipping non-selectable provider headers, and the dialog body scrolls so the new selection is visible.
  #   5. User in /resume session picker presses Home: selection jumps to the first session and the list scrolls to the top.
  #   6. User in /search history palette presses End: selection jumps to the last match and the list scrolls so it is visible on the bottom row.
  #   7. User clicks (left-button) on a row in /resume picker: the click row becomes the selected session and Enter immediately resumes it.
  #   8. User in ThinkingLevelDialog scrolls the trackpad wheel down: selection advances to the next level (wrapping to Off at the bottom).
  #   9. User scrolls the trackpad wheel over the underlying AgentView region OUTSIDE the open SlashCommandPopup rect: the popup ignores the event so the scrollback scrolls normally (popup hit-tests its last-rendered rect).
  #
  # ========================================

  Background: User Story
    As a fspec-tui user
    I want to navigate every selectable dialog/popup/picker with scroll, mouse-wheel, and wrap-around just like BoardView does
    So that no list view silently hides rows past index 9 and trackpad scroll works everywhere

  @slash-popup @scrolling
  Scenario: SlashCommandPopup scrolls the viewport when the selection moves past visible_rows
    Given the SlashCommandPopup is open with 14 matching commands and visible_rows is 10
    And the popup is at scroll_offset 0 with selected_index 0
    When the user presses Down 10 times
    Then the selected_index is 10
    And the scroll_offset has advanced so the selected row is inside the visible window
    And the top body row paints the "↑" glyph

  @slash-popup @wrap-around
  Scenario: SlashCommandPopup wraps from the last match back to the first on Down
    Given the SlashCommandPopup is open with 14 matching commands
    And the selected_index is at the last match (13)
    When the user presses Down
    Then the selected_index wraps to 0
    And the scroll_offset is reset to 0 so row 0 is visible

  @slash-popup @wrap-around
  Scenario: SlashCommandPopup wraps from the first match to the last on Up
    Given the SlashCommandPopup is open with 14 matching commands
    And the selected_index is 0
    When the user presses Up
    Then the selected_index wraps to 13
    And the scroll_offset advances so the last row is visible
    And the bottom body row stops painting the "↓" glyph

  @file-popup @mouse
  Scenario: FileSearchPopup mouse-wheel up moves the selection up one row
    Given the FileSearchPopup is open with 12 matches and visible_rows is 10
    And the selected_index is 5
    When the user emits MouseEventKind::ScrollUp inside the popup rect
    Then the selected_index decreases to 4
    And the popup remains visible with the selection inside the window

  @file-popup @mouse @wrap-around
  Scenario: FileSearchPopup mouse-wheel up at the first row wraps to the last match
    Given the FileSearchPopup is open with 12 matches and visible_rows is 10
    And the selected_index is 0 with scroll_offset 0
    When the user emits MouseEventKind::ScrollUp inside the popup rect
    Then the selected_index wraps to 11
    And the scroll_offset advances so the last row is visible

  @model-selector @page-keys
  Scenario: ModelSelectorDialog PageDown jumps by visible_rows and skips non-selectable headers
    Given the ModelSelectorDialog is open with 30 rows including provider headers
    And visible_rows is 12 and selected_index is on the first selectable row
    When the user presses PageDown
    Then the selected_index advances by visible_rows and lands on a selectable row
    And the scroll_offset has advanced so the new selection is visible
    And the bottom body row paints the "↓" glyph if more rows lie below

  @resume @page-keys
  Scenario: ResumeSessionView Home jumps to the first session and scrolls to the top
    Given the /resume session picker is open with 20 sessions and visible_rows is 8
    And the selected_index is 15 with scroll_offset 8
    When the user presses Home
    Then the selected_index is 0
    And the scroll_offset is 0
    And the top body row no longer paints the "↑" glyph

  @search @page-keys
  Scenario: SearchHistoryView End jumps to the last match and scrolls so it is visible
    Given the /search history palette has 25 matches and visible_rows is 10
    And the selected_index is 0 with scroll_offset 0
    When the user presses End
    Then the selected_index is 24
    And the scroll_offset has advanced so row 24 is on the bottom visible row
    And the top body row paints the "↑" glyph

  @resume @mouse
  Scenario: Left-click on a row in /resume picker selects that row
    Given the /resume session picker is open with 20 sessions and visible_rows is 8
    And the scroll_offset is 5 so rows 5..12 are visible
    When the user left-clicks on the second visible row
    Then the selected_index becomes 6
    And the row is highlighted with the inverse style

  @thinking-level @mouse @wrap-around
  Scenario: ThinkingLevelDialog mouse-wheel down advances and wraps at the last level
    Given the ThinkingLevelDialog is open with the High level selected
    When the user emits MouseEventKind::ScrollDown inside the dialog rect
    Then the selection wraps to Off
    And the dialog remains visible with the inverse highlight on the new row

  @slash-popup @mouse @hit-test
  Scenario: Mouse-wheel outside the SlashCommandPopup rect is ignored so the scrollback scrolls
    Given the SlashCommandPopup is open above AgentView's MultiLineInput
    And the mouse cursor is over the scrollback area, outside the popup rect
    When the user emits MouseEventKind::ScrollUp
    Then the popup returns EventResult::Ignored
    And the event bubbles to AgentView so the scrollback scrolls instead

  @shared-helper
  Scenario: scroll_viewport::wrap_index wraps in both directions using rem_euclid
    Given the shared scroll_viewport module is loaded
    When wrap_index(0, -1, 5) is called
    Then it returns 4
    And wrap_index(4, 1, 5) returns 0
    And wrap_index(2, 10, 5) returns 2

  @shared-helper
  Scenario: scroll_viewport::ensure_visible scrolls down when selected is past the window
    Given scroll_offset is 0 and visible_rows is 8 and total is 20
    When ensure_visible(&mut scroll_offset, 10, 8, 20) is called
    Then scroll_offset is updated so 10 lies in [scroll_offset, scroll_offset + 8)

  @shared-helper @wheel-velocity
  Scenario: WheelVelocity ramps up to 5x within 150ms then resets after the gap
    Given a fresh WheelVelocity
    When the user emits 5 ScrollDown events within 100ms of each other
    Then the 5th step reports velocity 5
    And after a gap of >=150ms the next step resets velocity to 1
