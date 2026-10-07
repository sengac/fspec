@done
@bug
@menu-bar
@tui-component
@board-view
@agent-view
@mux
@mouse-events
@keyboard-navigation
@BUG-197
Feature: Menu bar: click-away must de-select a focused item/chip (all 3 surfaces) and stale ring focus leaves chips highlighted after view flips

  """
  Three surfaces, three ring-focus holders: Board = BoardStore.menu_focus (store/board_menu.rs; MenuDismissBar clears it); Agent = AgentView.menu_state (plain field; menu_state.clear_focus in-view); Mux = MultiplexLayout.menu_focus (menu_dismiss arm). All three paint through the shared components/menu_bar painter; the chip highlight decision is the single focused_cell() in components/menu_bar/paint.rs (ring focus OR MenuChip.active — MENU-010: the per-chip flag is set by the SURFACE's snapshot builder, never on the board).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: A left click OUTSIDE the open dropdown closes the dropdown AND clears the bar's ring focus — extending BUG-196 R3 to ALL bar focus states, not just open-dropdown: the board store's MenuDismissBar arm, the agent view's in-view clear, and the mux layout's arm all run whenever a left click lands OFF the bar row (on content/panes), whether or not a dropdown is open.
  #   2. R2: A left click on the bar row that hits NO zone (empty bar space) clears the bar's ring focus — it does NOT leave the bar engaged. Applies to all three surfaces: board (MenuDismissBar or in-view clear), agent (menu_state.clear_focus), mux (menu_dismiss). The click is otherwise inert (no selection change, no focus landing).
  #   3. R3: On the board, when a chip activation (MenuChipActivate) flips the active view to the Agent view, the board store's bar ring focus (menu_focus) must be cleared so that returning to the board does not show a stale chip highlight. The focused column / card selection is preserved.
  #   4. R4: A session chip may carry the inverse-video highlight ONLY when (a) the arrow-key ring selector (MenuFocus::ZoneB) is on that chip, or (b) the surface's snapshot builder marked it as the current-session chip (MENU-010: the Agent view's store-current session, the mux's focused Agent pane — NEVER on the board). No other highlight path is permitted — the fix achieves this by eliminating the STALE ring-focus states (R1-R3), NOT by changing the MENU-006/MENU-010 painter logic.
  #
  # EXAMPLES:
  #   1. Board: Kanban item focused (dropdown CLOSED). Left click on a work-unit card in another column → card selected, bar highlight clears (menu_focus None, focused_column = clicked column). Pressing '.' afterwards opens the new-agent flow (key binding live again).
  #   2. Board: 2 open sessions, ring selector walked onto chip #2 (item highlight on the chip). Left click a card in the BACKLOG column → the card is selected, the chip highlight disappears, and returning via keyboard walks a fresh ring (no chip pre-highlighted).
  #   3. Board: ring selector on chip #1, click the chip → jump into that session's Agent view. Return to the Board (Esc cascade or BackToBoard). The board bar must NOT pre-highlight chip #1 — MENU-010: the board carries no current-session chip highlight at all.
  #   4. Agent view: ring selector walked onto the 'Board View' item or a chip (highlight visible). Left click on empty bar-row space (between zones) → the highlight clears; the input composer regains keyboard focus. No session switch, no view flip.
  #   5. Agent view: ring selector on chip #2, activate 'Board View' (Enter or click) → flip to the Board view. The board bar must not show a pre-highlighted chip — MENU-010: the board carries no current-session chip highlight at all.
  #   6. Mux: a bar item focused with its dropdown CLOSED (e.g. 'Kanban' highlighted). Left click the Files pane → the Files pane gains focus AND the bar highlight clears. Keyboard bindings of the focused pane are live again immediately.
  #   7. Mux: ring selector on a chip (highlight visible). Left click empty bar-row space (no item, no cell hit) → the chip highlight clears; the focused pane keeps its focus (the click does not land on a pane).
  #
  # ========================================

  Background: User Story
    As a TUI user driving the menu bar with mouse and arrow keys
    I want to click away from a focused bar item/chip to de-select it, and never see a chip highlighted unless my selector is on it or it is my active session
    So that the bar highlight always reflects my actual selection, so I can tell at a glance which item/chip is engaged

  # ========================================
  # SCENARIOS
  # ========================================
  Scenario: Board: clicking away from a focused menu item de-selects it
    Given the board is focused on a kanban column and the Kanban bar item is focused with its dropdown closed
    When I left click a work-unit card in another column
    Then the clicked card becomes selected in its column
    And the bar highlight clears (no menu item is focused)

  Scenario: Board: clicking away from a ring-focused chip de-selects it
    Given the board has 2 open sessions and the ring selector is on chip #2 (chip highlight visible)
    When I left click a card in the BACKLOG column
    Then the clicked card becomes selected in the BACKLOG column
    And the chip highlight disappears (no chip is ring-focused)

  Scenario: Board: key bindings are live again after clicking away from the bar
    Given the board has no open sessions and the Kanban bar item is focused
    When I left click a work-unit card in a column and then press the '.' key
    Then the create-session dialog opens (the '.' binding ran and was not swallowed by the bar)

  Scenario: Board: returning to the board after a chip activation shows no pre-highlighted chip
    Given the board has 2 open sessions and the ring selector is on chip #1
    When I click chip #1 to open its Agent view and then return to the board
    Then the board bar does not pre-highlight any chip with the ring selector (no chip is ring-focused)
    And no chip's cells paint the inverse-video highlight (the board never carries the current-session highlight — MENU-010)

  Scenario: Agent view: clicking empty bar space clears the bar highlight
    Given the agent view shows a session and the ring selector is on a bar item or chip (highlight visible)
    When I left click empty bar-row space (no item, no chip, no dropdown open)
    Then the bar highlight clears and the input composer regains keyboard focus

  Scenario: Agent view: flipping back to the board leaves no ring-focused chip
    Given the agent view shows a session, the ring selector is on a chip, and its session is the current session
    When I activate the 'Board View' item (Enter or click) to flip to the board
    Then the board bar does not show a pre-highlighted chip (no chip is ring-focused)
    And no chip's cells paint the inverse-video highlight (the board never carries the current-session highlight — MENU-010)

  Scenario: Mux: clicking a pane clears the bar highlight
    Given the mux surface shows a board pane and a Files pane, and a bar item is focused with its dropdown closed
    When I left click the Files pane
    Then the Files pane gains focus and the bar highlight clears

  Scenario: Mux: clicking empty bar space clears the chip highlight
    Given the mux surface has a focused pane, and the ring selector is on a chip (chip highlight visible)
    When I left click empty bar-row space (no item, no chip, no cell hit)
    Then the chip highlight clears
    And the focused pane keeps its focus (the click does not land on a pane)
