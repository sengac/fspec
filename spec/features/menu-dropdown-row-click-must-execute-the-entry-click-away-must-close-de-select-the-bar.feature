@done
@bug
@menu-bar
@tui-component
@board-view
@mouse-events
@keyboard-navigation
@BUG-196
Feature: Menu dropdown: row click must execute the entry; click away must close + de-select the bar
  """
  Architecture notes:
  - One pure dropdown click hit-test in the shared menu_bar component
  (`dropdown_row_at`) — the scroll-window math the painter already uses,
  so the hit-test can never drift from the paint.
  - The board and mux mouse arms share it (DRY): left click over the open
  panel → `MenuExecuteItem{category,row}`; inside-panel miss → swallow.
  - A new `MenuDismissBar` bus token expresses "close the dropdown AND
  clear the bar's ring focus" as one intent; the board store arm and the
  mux layout arm each implement their own holder semantics.
  - No regression: Esc (keyboard close) keeps the parent item focused;
  only the click-away path de-selects the bar (R4).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The dropdown row hit-test is ONE pure function in the shared menu_bar component (dropdown_row_at): given the open panel rect, category and cursor it returns the registry row index under a (column, row) click. It reuses the painter's existing scroll-window math (scroll_window + visible_entry_rows) so the hit-test can never drift from the paint; clicks on border rows, the dim ellipsis row, and positions outside the panel map to no row. All surfaces (board, mux) route left clicks on the open panel through this helper — DRY/SOLID, no per-surface duplication of the row math.
  #   2. R2: A left click on an open dropdown's entry row executes that row on the SAME event: the panel closes, the bar focus clears (board store and mux layout), and the resolved MenuAction is re-dispatched on the bus through the existing MenuExecuteItem path — exactly what Enter on the highlighted row does today. Clicking a non-cursor row executes THAT row, not the cursor row (GUI parity: click selects + confirms). The board store arm and the mux layout arm already resolve rows through the MenuCategories registry; no new Action variant is needed for execution.
  #   3. R3: A left click OUTSIDE the open dropdown (off the bar row, off the panel) closes the dropdown AND clears the bar's ring focus (de-selects the parent menu item) so the surface's key bindings are live again immediately. The click STILL lands on the surface it hit (a kanban card in the board view, a pane in mux — the existing outside-click 'the click still lands' behavior is preserved). The new MenuDismissBar bus token expresses 'close + de-select' as one intent: the board store arm closes the dropdown and drops the bar focus (focused column mirrors as before); the mux arm closes the dropdown and drops the mux bar focus. A bar-row click that hits no item/cell still swallows as today (no focus change, no de-select).
  #   4. R4 (no regression): Esc with the dropdown open still closes the dropdown AND keeps the parent item focused (MENU-002 R9 / MENU-004 parity — the keyboard close is a 'stay on the bar' gesture, the click-away is a 'leave the bar' gesture). The board store's `close_menu()` and the mux's `menu_close_dropdown()` are unchanged; only the outside-click path gains the de-select.
  #
  # EXAMPLES:
  #   1. Board view: with the Kanban dropdown open and the cursor on row 0, a left click on the row painted at panel.y+2+1 (the '/ Search' entry, panel inner x+1..right-1) opens the work-unit search dialog, the dropdown closes, and no bar highlight remains (menu_focus None, focused column unchanged).
  #   2. Board view: with the Kanban dropdown open, a left click on a work-unit card in another column closes the dropdown, de-selects the Kanban item from the bar, and selects that card — after the click, pressing '.' opens the new-agent flow (the '.' board key binding is live again, not swallowed by the bar).
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to drive the menu dropdowns with the mouse
    So that I can select a menu entry by clicking it and regain my key bindings by clicking away

  # ========================================
  # SCENARIOS
  # ========================================
  Scenario: Clicking a dropdown entry row executes that entry
    Given the board has 1 open session and the Kanban dropdown is open with the cursor on row 0 (New Agent)
    When I click the 'New Agent' row in the open dropdown
    Then the active view is the Agent view showing that session
    And the dropdown closes and the bar highlight clears

  Scenario: Clicking a non-cursor dropdown row executes the clicked row
    Given the board has 2 open sessions and the Kanban dropdown is open with the cursor on row 3 (Attachments)
    When I click the 'New Agent' row (row 0) in the open dropdown
    Then the active view is the Agent view showing the session that was current before the click
    And the dropdown closes and the bar highlight clears

  Scenario: Clicking the Help dropdown Exit row opens the exit confirmation
    Given the Help dropdown is open with the cursor on row 0 (Help)
    When I click the 'Exit' row in the open dropdown
    Then the exit confirmation dialog opens
    And the dropdown closes and the bar highlight clears

  Scenario: A click on the dropdown border rows does nothing
    Given the Kanban dropdown is open with the cursor on row 0
    When I click the panel's top border row
    Then the dropdown stays open with the cursor on row 0

  Scenario: Clicking away closes the dropdown and de-selects the bar
    Given the Kanban dropdown is open and the board is focused on a kanban column
    When I click a work-unit card in another column
    Then the dropdown closes and that card becomes selected in its column
    And the bar highlight clears (no menu item is focused)

  Scenario: The board key bindings are live again after a click away
    Given the Kanban dropdown is open and the board has no open sessions
    When I click a work-unit card in another column and then press the '.' key
    Then the create-session dialog opens (the '.' binding ran and was not swallowed by the bar)

  Scenario: Clicking a mux dropdown row executes that entry
    Given the mux 'Settings' dropdown is open with the cursor on row 0 (Mux)
    When I click the 'Mux' row in the open dropdown
    Then the mux config dialog opens
    And the dropdown closes and the mux bar highlight clears

  Scenario: Clicking away in mux closes the dropdown and de-selects the bar
    Given the mux 'Kanban' dropdown is open
    When I click the Files pane (outside the dropdown panel and off the bar items)
    Then the dropdown closes AND the Files pane is focused (the click lands on the same event)
    And the mux bar highlight clears (the parent item is de-selected)

  Scenario: A mux click on the dropdown border rows does nothing
    Given the mux 'Kanban' dropdown is open with the cursor on row 0
    When I click the panel's top border row
    Then the dropdown stays open with the cursor on row 0

  Scenario: Esc still closes the dropdown and keeps the item focused
    Given the Kanban dropdown is open
    When I press Esc once
    Then the dropdown is closed and the Kanban item stays focused
