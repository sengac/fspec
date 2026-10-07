@done
@menu-bar
@tui-component
@ui-enhancement
@MENU-006
Feature: Active session chip paints the selected-item blue background

  """
  The highlight is a per-chip `active` flag carried on `MenuChip` (components/menu_bar/chips.rs). MENU-010 moved the decision into the SURFACE's snapshot builder: the agent view builder marks the store's current session's chip (views/agent/menu_render.rs), the mux builder marks only when an Agent pane is focused (views/multiplex/menu_snapshot.rs via MultiplexLayout::focused_session_id), and the board builder marks no chip (views/board/menu_snapshot.rs `chip_inputs` always `active=false` + `mark_active_chip` helper). The shared painter (components/menu_bar/paint.rs `focused_cell`) ORs the per-chip flag with the ring focus and, when true, paints the whole cell inverse-video (bg Cyan, fg Black, bold) — the same `inverse_style()` used for selected Zone A items. All three surfaces flow through this one painter so no per-surface paint code is needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On an active chip the '#n' prefix, the work-unit id suffix (when level 0) and the status glyph all paint in the inverse style (fg Black, bg Cyan, bold) — the colored glyph/dim WU variants do not paint on top of the highlight.
  #   2. The selected-item highlight paints only where it means something (MENU-010 R1): in the Agent view, the chip whose session is the store's CURRENT session; in mux mode, the chip of the session rendered in the FOCUSED Agent pane (agent-pane parity). The BOARD view carries no current-session chip highlight (MENU-010 R2 — this feature's old board scenario is superseded and now pins the NO-highlight behavior).
  #   3. The active highlight is independent of the ring focus on the surfaces that carry it (agent view): it stays painted when the menu bar is NOT focused (no keyboard/mouse ring), and when the ring IS focused on the active chip the cell still paints the same inverse style (no double/highlight conflict).
  #   4. At most one surface-driven chip carries the active highlight: the surface's snapshot builder marks at most one chip (the current session / focused agent pane). All other chips keep their normal #333333 background. If that session is not in the painted (non-Cleared) chip list, no chip is highlighted. The ring-selector path (MenuFocus::ZoneB) is surface-independent (MENU-010 R3) and may additionally highlight the ring's chip.
  #   5. Shared painter: the highlight is decided in the pure MenuSnapshot (a per-chip active flag) and painted by components/menu_bar/paint.rs, so the agent view and the mux surface both get it with no per-surface paint code.
  #
  # EXAMPLES:
  #   1. Agent view, 3 open sessions, s-2 is the focused session, the bar is rendered with NO ring focus: chip #2's cells carry bg Cyan fg Black (like a selected 'Actions' item); chips #1 and #3 keep the dark #333333 background.
  #   2. In the Agent view showing s-2 (3 sessions), the row reads ' Board View │ #1 ●  #2 ●  #3 ●' where '#2 ●' is painted inverse-video; click away to the input (ring cleared) and the chip stays blue.
  #   3. The active chip is focused in the ring at level 0 with a work-unit id: the whole cell — '#n', WU id and glyph — paints fg Black bg Cyan bold (no magenta/cyan status glyph on top).
  #   4. Mux mode, [Board | Agent] with s-1 and s-2 open and the Agent pane focused showing s-1: the Board view label keeps its normal styling, chip #1 paints inverse-video, chip #2 stays dark (the menu-bar-click-away feature's MENU-010 scenarios pin the full mux rule).
  #
  # ========================================

  Background: User Story
    As a TUI user
    I want to see the currently-active session's chip highlighted with the selected-menu-item blue background on the Agent surface
    So that I always know which session I am looking at (MENU-010: the board view no longer carries this highlight)

  @menu-focus
  Scenario: The active session's chip keeps its blue background in the Agent view after the ring focus clears
    Given the App is in the Agent view with 3 open sessions and session s-2 is the focused session
    When the menu bar ring focus is cleared
    Then chip #2's cells in the agent bar are styled bg Cyan fg Black
    And chip #1's and chip #3's cells keep the #333333 background


  Scenario: The board view does not carry the current session's chip highlight (superseded by MENU-010)
    Given the board has 3 open sessions and session s-2 is the focused session
    When the bar renders with the ring focus cleared
    Then no chip's cells are styled bg Cyan fg Black (the board never carries the current-session highlight — MENU-010)
    And chips #1 and #3's cells keep the #333333 background


  @menu-focus
  Scenario: A ring-focused active chip with a work-unit id paints the whole cell inverse
    Given a MenuSnapshot with 3 open sessions and session s-2 is the focused session and chip #2 is bound to work unit "MENU-006" and the menu bar ring is focused on chip #2
    When the bar is rendered into a 120-column row
    Then chip #2's whole cell (prefix, work-unit id and glyph) is styled bg Cyan fg Black


  @menu-focus
  Scenario: In mux mode the focused Agent pane's session chip paints the blue background while view labels keep their styling
    Given a MenuSnapshot in mux mode with panes Board and Agent, 2 open sessions, and the Agent pane is focused showing session s-1
    When the bar is rendered into a 120-column row
    Then chip #1's cells are styled bg Cyan fg Black
    And chip #2's cells keep the #333333 background

