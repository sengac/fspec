@done
@bug
@menu-bar
@tui-component
@board-view
@keyboard-navigation
@BUG-198
Feature: Menu dropdown cycles with the board ring walk — Left/Right re-anchors the open panel
  """
  The mux bar already does this (MENU-004 R-KEYS, `MultiplexLayout::menu_reanchor_or_close`): while a dropdown is open, Left/Right walk the ring and an item landing re-anchors the panel under the newly focused item at row 0, a Zone B landing closes it. The board surface's `BoardStore::menu_move` (store/board_menu.rs) walked the ring WITHOUT touching `menu_open`, so the open panel stayed painted under the stale item (Kanban) while the highlight moved to Tools/Settings/Help — and the board's R7 wheel L/R arms (the same `MenuMove` dispatch) inherited the mismatch. Fix: the same re-anchor rule in the store — an item landing moves (or opens) the panel at row 0; landing off the items (a column or a chip) closes it. The agent view is dropdown-free (MENU-007) — untouched.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: While the board bar's dropdown is open, a Left/Right (h/l) ring walk re-anchors the panel on every landing: an item landing moves the open panel under that item with the cursor re-armed at row 0; a landing off the items (a kanban column or a session chip) closes the panel. This is the board mirror of the mux surface's `menu_reanchor_or_close` and the GUI menu-bar behavior (cycling items with the arrows keeps the dialog open and re-anchors it).
  #   2. R2: The wheel L/R ring walks over the bar row (R7, `MenuMove` dispatch) follow the SAME re-anchor rule — the wheel and the keys are parity by construction (both arms dispatch `Action::MenuMove`).
  #   3. R3 (no regression): with NO dropdown open the ring walk is byte-for-byte unchanged (columns ⇄ items ⇄ chips, the `menu_open` stays `None`); Esc keeps its MENU-002 R9 semantics (close, item stays focused); Enter-on-open-dropdown executes + clears (BUG-196 R2); click-away de-selects (BUG-196 R3 / BUG-197).
  #
  # EXAMPLES:
  #   1. Kanban dropdown open, cursor on row 1 (Search): Right → the Tools item holds the highlight and the Tools panel (Changed Files / Checkpoints) paints under Tools with the cursor on row 0; Left → the panel re-anchors under Kanban again at row 0.
  #   2. Help dropdown open, 1 open session: Right walks onto chip #1 and the panel closes; Left walks back onto Help and the Help panel re-opens under Help at row 0.
  #   3. Kanban dropdown open: Left from the first item walks into the last column (blocked) — the bar highlight clears, the column keeps its selection, and the panel closes.
  #
  # ========================================
  Background: User Story
    As a TUI user on the board surface
    I want to cycle the open dropdown between menu items with Left/Right
    So that the dropdown follows my cursor like a GUI menu bar, on the board exactly as on the mux surface

  # ========================================
  # SCENARIOS
  # ========================================
  Scenario: While a dropdown is open Left and Right walk the ring and re-anchor
    Given the Kanban dropdown is open with the cursor on row 1 (Search)
    When I press Right once and then Left once
    Then the dropdown re-anchors under the focused item at row 0 each time and the panel paints under the new item

  Scenario: While a dropdown is open Left walks the ring back onto the previous item
    Given the Kanban dropdown is open and the ring has walked onto the Tools item
    When I press Left once
    Then the Tools item loses the focus and the dropdown re-anchors under the Kanban item at row 0

  Scenario: While a dropdown is open Left from the first item walks into the columns and closes it
    Given the Kanban dropdown is open with the ring on the first item
    When I press Left once
    Then the focused column is re-focused, the bar highlight clears and the dropdown closes

  Scenario: While a dropdown is open Right from the last item walks onto a chip and closes it
    Given the Help dropdown is open and the board has 1 open session
    When I press Right once
    Then the first chip has the focus and the dropdown closes

  Scenario: Wheel left and right over the bar with an open dropdown re-anchors the panel
    Given the Kanban dropdown is open and the board has 1 open session
    When I scroll the wheel right once over the bar row
    Then the Tools item has the focus and the open dropdown re-anchors under it at row 0
