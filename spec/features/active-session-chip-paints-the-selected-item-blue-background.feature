@done
@menu-bar
@tui-component
@ui-enhancement
@MENU-006
Feature: Active session chip paints the selected-item blue background

  """
  The highlight is a per-chip `active` flag carried on `MenuChip` (components/menu_bar/chips.rs). The pure snapshot builder (views/board/menu_snapshot.rs) sets `active=true` on the chip whose session equals `store.current_session()`. The shared painter (components/menu_bar/paint.rs `paint_cell`) checks the flag first and, when true, paints the whole cell inverse-video (bg Cyan, fg Black, bold) — the same `inverse_style()` used for selected Zone A items. All three surfaces (board, agent, mux) flow through this one painter so no per-surface code is needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On an active chip the '#n' prefix, the work-unit id suffix (when level 0) and the status glyph all paint in the inverse style (fg Black, bg Cyan, bold) — the colored glyph/dim WU variants do not paint on top of the highlight.
  #   2. The Zone B session chip whose session is the CURRENTLY ACTIVE (focused) session paints its whole cell with the selected-menu-item style: bg Cyan, fg Black, bold — the exact style a selected Zone A item (Actions/Help) uses (the user calls this the blue background).
  #   3. The active highlight is independent of the ring focus: it stays painted when the menu bar is NOT focused (no keyboard/mouse ring), and when the ring IS focused on the active chip the cell still paints the same inverse style (no double/highlight conflict).
  #   4. Exactly one chip carries the active highlight: the snapshot builder marks the chip whose session equals the store's current session; all other chips keep their normal #333333 background. If the current session is not in the painted (active) chip list, no chip is highlighted.
  #   5. Shared painter: the highlight is decided in the pure MenuSnapshot (a per-chip active flag) and painted by components/menu_bar/paint.rs, so the board, agent and mux surfaces all get it with no per-surface code.
  #
  # EXAMPLES:
  #   1. 3 open sessions, s-2 is the focused session, the bar is rendered with NO ring focus: chip #2's cells carry bg Cyan fg Black (like a selected 'Actions' item); chips #1 and #3 keep the dark #333333 background.
  #   2. In the Agent view showing s-2 (3 sessions), the row reads ' Actions Help │ #1 ●  #2 ●  #3 ●' where '#2 ●' is painted inverse-video; click away to the input (ring cleared) and the chip stays blue.
  #   3. The active chip is focused in the ring at level 0 with a work-unit id: the whole cell — '#n', WU id and glyph — paints fg Black bg Cyan bold (no magenta/cyan status glyph on top).
  #   4. Mux mode, [Board | Agent | Files] with s-1 and s-2 open and s-2 focused: the Board and Files view labels keep their normal styling, chip #2 paints inverse-video, chip #1 stays dark.
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to see the currently-active session's chip highlighted with the selected-menu-item blue background
    So that always know which session I am looking at, on any surface (board, agent, mux)

  @menu-focus
  Scenario: The active session's chip keeps its blue background in the Agent view after the ring focus clears
    Given the App is in the Agent view with 3 open sessions and session s-2 is the focused session
    When the menu bar ring focus is cleared
    Then chip #2's cells in the agent bar are styled bg Cyan fg Black
    And chip #1's and chip #3's cells keep the #333333 background


  @menu-focus
  Scenario: In mux mode the active session's chip paints the blue background while view labels keep their styling
    Given a MenuSnapshot in mux mode with panes Board and Files, 2 open sessions, and session s-2 is the focused session
    When the bar is rendered into a 120-column row
    Then chip #2's cells are styled bg Cyan fg Black
    And chip #1's cells keep the #333333 background


  @menu-focus
  Scenario: The active session's chip paints the selected-item blue background without ring focus
    Given the board has 3 open sessions and session s-2 is the focused session
    When the bar is rendered
    Then chip #2's cells are styled bg Cyan fg Black
    And chips #1 and #3's cells keep the #333333 background


  @menu-focus
  Scenario: A ring-focused active chip with a work-unit id paints the whole cell inverse
    Given a MenuSnapshot with 3 open sessions and session s-2 is the focused session and chip #2 is bound to work unit "MENU-006" and the menu bar ring is focused on chip #2
    When the bar is rendered into a 120-column row
    Then chip #2's whole cell (prefix, work-unit id and glyph) is styled bg Cyan fg Black

