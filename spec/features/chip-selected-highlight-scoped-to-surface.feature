@done
@menu-bar
@agent-view
@board-view
@mux
@menu-chips
@MENU-010
Feature: Chip selected-highlight scoped to surface

  """
  The per-chip `active` flag (MenuChip in components/menu_bar/chips.rs) is the single source of the selected-item highlight on the shared 2-zone bar painter (components/menu_bar/paint.rs `focused_cell` — ring focus OR `MenuChip.active`). MENU-010 moves the decision into the SURFACE's snapshot builder: the shared `chip_inputs` builder (views/board/menu_snapshot.rs) no longer marks any chip active. The agent view builder (views/agent/menu_render.rs `agent_bar_snapshot`) marks the store's current session's chip via `mark_active_chip`; the mux builder (views/multiplex/menu_snapshot.rs `build_snapshot`) marks only when an Agent pane is focused — `MultiplexLayout::focused_session_id()` (None for a Board/Files/Ckpts-focused pane); the board builder never marks. `mark_active_chip` is a no-op when the session is not in the painted (non-Cleared) chip list. The ring-selector path (MenuFocus::ZoneB) is untouched — surface-independent per R3. Supersedes the MENU-006 board and mux scenarios (the store's current session no longer drives the highlight on those surfaces — see active-session-chip-paints-the-selected-item-blue-background.feature, updated).
  """

  Background: User Story
    As a TUI user
    I want to see the session chip carry the selected-item (blue) highlight only where it means something — the current agent in the Agent view (and the focused agent pane in mux), or the chip my ring selector is on
    So that the board view no longer shows a stale blue highlight on the last active agent's chip after flipping back to the board


  Scenario: The board view does not highlight the current session's chip after flipping back from the Agent view
    Given the board has 3 open sessions and session s-2 is the store's current session
    When I open session s-2's Agent view from the board and then return to the board with the ring focus cleared
    Then no cell on the board bar row paints the inverse-video highlight (chip #2's cells keep the #333333 background)


  Scenario: The ring selector highlights a chip on the board view
    Given the board has 3 open sessions and the ring selector (MenuFocus::ZoneB) is on chip #2
    When the board view renders the 2-zone menu bar
    Then chip #2's cells are styled bg Cyan fg Black (the ring selector path) and chip #1's and chip #3's cells keep the #333333 background


  Scenario: The Agent view highlights the current session's chip without a ring focus
    Given the App is in the Agent view with 3 open sessions and session s-2 is the store's current session
    When the Agent view's menu bar renders with no ring focus
    Then chip #2's cells are styled bg Cyan fg Black and chip #1's and chip #3's cells keep the #333333 background


  Scenario: The ring selector highlights a chip on the Agent view
    Given the App is in the Agent view with 3 open sessions, session s-2 is the store's current session, and the ring selector is on chip #1
    When the Agent view's menu bar renders
    Then chip #1's cells are styled bg Cyan fg Black (ring selector) and chip #2's cells are styled bg Cyan fg Black (the current session's chip) while chip #3's cells keep the #333333 background


  Scenario: Mux: the focused Agent pane's session chip is highlighted
    Given the mux surface has panes Board and Agent with 2 open sessions, and the Agent pane is focused showing session s-1
    When the mux bar renders with no ring focus
    Then chip #1's cells (the focused Agent pane's session) are styled bg Cyan fg Black and chip #2's cells keep the #333333 background


  Scenario: Mux: no chip is highlighted while a non-Agent pane is focused
    Given the mux surface has panes Board and Agent with 2 open sessions, and the Board pane is focused
    When the mux bar renders with no ring focus
    Then chip #1's and chip #2's cells keep the #333333 background

