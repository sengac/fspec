@done
@menu-bar
@tui-component
@keyboard-navigation
@board-view
@MENU-002
Feature: Board surface — header row replaced by the 2-zone bar + column→menu→chip continuous ring (keys, wheel, mouse)
  """
  Actions: new Action variants in components/mod.rs — MenuMove(i32), MenuFocusToColumns, MenuOpenDropdown(usize), MenuCloseDropdown, MenuDropdownCursor(i32), MenuExecuteItem{category,row}, MenuChipActivate(usize), MenuMoveToItem(usize); all consumed by App::dispatch, board view only emits. BoardStore gains menu_focus: Option<MenuFocus> + set_menu_ring_size() + menu_focus_prev/next() with wrap math on the store (mirrors focus_next_column); dispatch feeds ring size = 7 + items + open_sessions().len().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: The board's kanban navigation becomes the front half of one continuous wrap-around ring walked by Left/Right (h/l) and wheel ScrollLeft/ScrollRight: the 7 kanban columns, then the menu items, then the session chips, then back to the first column. Right on the last column lands on the first menu item (dropdown closed, item highlighted); left on the first column lands on the last chip. Columns, items and chips each occupy exactly one ring stop.
  #   2. R1: The board header's row-3 'u Actions' chord is replaced by the live 2-zone MenuBar (Zone A menu items + Zone B session chips, chips-only in non-mux mode) painted into the same right-column rect; the 'u Actions' chord string no longer renders on the board header.
  #   3. R3: Up/Down (k/j) from a menu item or chip focus drops focus back into the board grid: the focused column is re-focused, its selection is unchanged, and the bar highlight clears. Up/Down on a column behaves exactly as before (SelectPrev/Next card).
  #   4. R5: Enter on a focused session chip jumps to that session by emitting Action::OpenAgentView(Some(session_id)), flipping the active view to the Agent view via App::dispatch. A chip exists if and only if its session is open, so chip activation always jumps to an existing session; creating a new agent remains the '.'/New Agent menu entry.
  #   5. R4: Enter on a focused menu item toggles/advances its dropdown: if the item's dropdown is closed, Enter opens it with the cursor on row 0; if it is open, Enter executes the highlighted row (resolving through the MenuCategories registry to the existing Action, then closing the dropdown). Enter on a column is unchanged (EnterWorkUnit / MuxEnterWorkUnit).
  #   6. R6: While a menu dropdown is open on the board the bar is true-modal: all other keys (including j/k/h/l, bare shortcut letters, Enter on columns) are swallowed and Up/Down (wheel ScrollUp/ScrollDown) move the dropdown cursor between rows, wrapping at the ends; clicking outside the dropdown (or on the open item again, GUI parity) closes it.
  #   7. R8: Ring sizing is dispatch-fed: App::dispatch computes the chip count from agent_view_store.open_sessions() and calls board_store.set_menu_ring_size(columns + menu_items + chips), so the board store stays session-order-agnostic. The store owns the ring position and the wrap math (menu_focus_prev/next mirrors focus_prev/next_column), keeping handle_event thin.
  #   8. R7: Mouse on the bar: clicking a menu item focuses it AND opens its dropdown (one gesture; clicking the already-open item closes it — GUI parity); clicking a chip activates it (same as Enter on the chip); wheel ScrollRight/ScrollLeft over the bar row walks the ring exactly like the keys; wheel over the bar with no dropdown open is ignored; the bar rect hit-test runs before the details-strip and content tests.
  #   9. R10: BoardView::render_with_store gains an &AgentViewStore parameter (the Navigator call site already holds both stores); the board render helper builds the MenuSnapshot per frame (open sessions + focused session + clock_ms), paints the bar into the header's right-column row-3 rect, then caches item/chip rects in a Cell for mouse hit-testing; the dropdown is painted after the board box (anchored, clamped, NOT a Compositor layer).
  #   10. R9: Escape semantics: with the dropdown open, Esc closes the dropdown (item focus remains); with the dropdown closed, Esc is unchanged (App-level exit cascade — the bar has no Esc-clears-highlight behavior). Up/Down is the way to leave the bar.
  #   11. R11: The board footer hint and the board help (u) content rows are updated in this unit where they describe the old navigation: Left/Right now reads 'Cycle columns -> menu items -> chips' and an 'Enter on menu item opens dropdown / Enter on chip opens session' row is added (the full help-dialog content migration itself is MENU-005's scope).
  #
  # EXAMPLES:
  #   1. With 3 open sessions, at the last column (BLOCKED) pressing Right lands on the 'Actions' item (inverse-video, dropdown closed); Right again lands on 'Help'; Right again lands on chip #1; Right again on chip #2; Right again on chip #3; Right wraps back into the first column (BACKLOG). With ONE open session the walk is Actions → Help → chip #1 → back to the first column. Left from the first column (backlog) always lands on the LAST chip (or the last item when no chips).
  #   2. Clicking chip '#2' (a Running session) immediately flips the active view to that session's Agent view (same path as pressing Enter on the chip); clicking the 'Help' item opens its 2-row dropdown; clicking anywhere on a kanban cell with the dropdown open closes the dropdown and also performs the normal column/card selection.
  #   3. Focused on 'Actions', pressing Enter opens the dropdown under the item (cursor on row 0); pressing Enter again executes row 0 ('.' New Agent — the session-target snapshot is substituted at open time) and closes the dropdown; pressing Up/Down then Left from the bar returns focus to the last column with the previous card selection intact.
  #   4. With zero open sessions the bar shows only 'Actions Help' (no separator, no chips); the ring then is columns + items only and wraps from the last item straight back to the first column; clicking the bar's item area still works.
  #   5. Scrolling the wheel right while on the last column focuses the first menu item exactly like the Right key does; wheel left from the first column focuses the last chip; wheel over the bar with the dropdown closed is ignored (no ring movement), and wheel up/down over an open dropdown moves the cursor row by row.
  #
  # ========================================
  Background: User Story
    As a TUI developer
    I want to navigate the board via a continuous column→menu-item→chip ring rendered as the live 2-zone menu bar in the header's row 3
    So that the board's 'u Actions' chord becomes a discoverable, clickable, wheel-driven GUI menu bar wired to the existing actions and sessions

  Scenario: Left from the first column focuses the last chip
    Given the board has 3 open sessions and the first column (backlog) is focused
    When I press Left once
    Then chip #3 paints inverse-video

  Scenario: Enter on an open dropdown executes the highlighted row
    Given the Actions dropdown is open with the cursor on row 2 (Checkpoints)
    When I press Enter once
    Then the Checkpoints view opens and the dropdown closes

  Scenario: Enter on a chip opens that session agent view
    Given the board has 3 open sessions and the menu bar is focused on chip #2
    When I press Enter once
    Then the active view flips to session #2's agent view

  Scenario: Up and Down from the bar drop focus back into the focused column
    Given the board has 3 open sessions and the menu bar is focused on chip #1
    When I press Up once
    Then the focused column keeps its previous card selection and the bar highlight clears

  Scenario: Enter on a menu item opens its dropdown at row 0
    Given the board is focused on the Actions menu item with its dropdown closed
    When I press Enter once
    Then the Actions dropdown panel paints under the item with its cursor on row 0 (New Agent)

  Scenario: The ring walks items then chips then wraps to the first column
    Given the board has 1 open session and the last column (blocked) is focused
    When I press Right four times
    Then the focus lands on Actions then Help then chip #1 and finally back on the first column (backlog)

  Scenario: The board header row 3 paints the 2-zone menu bar instead of the u Actions chord
    Given the board has 2 open sessions (one running, one idle)
    When the board view renders
    Then row 3 of the header right column shows the menu items (Actions Help) a separator and the two session chips and the string 'u Actions' appears nowhere on the board

  Scenario: Right from the last column focuses the first menu item
    Given the board has no open sessions and the last column (blocked) is focused
    When I press Right once
    Then the Actions item paints inverse-video with its dropdown closed

  Scenario: The board footer hint describes the ring
    Given the board has 3 open sessions
    When the board view renders
    Then the footer hint row reads that Left Right cycle columns then menu items then chips

  Scenario: The header bar omits the separator and chips when no sessions are open
    Given the board has no open sessions
    When the board view renders
    Then the bar row shows only the menu items (Actions Help) with no separator and no chips

  Scenario: Esc closes the open dropdown and keeps the item focused
    Given the Actions dropdown is open and the cursor is on row 3
    When I press Esc once
    Then the dropdown is closed and the Actions item stays focused

  Scenario: Clicking a menu item focuses it and opens its dropdown
    Given the board is focused on a kanban column
    When I click the Help menu item in the bar
    Then the Help item paints inverse-video and its dropdown opens with the cursor on row 0

  Scenario: With no open sessions the ring wraps from the last item to the first column
    Given the board has no open sessions and the menu bar is focused on the last menu item (Help)
    When I press Right once
    Then the first column (backlog) is focused again and the bar highlight clears

  Scenario: While a dropdown is open all other keys are swallowed and Up Down move the cursor
    Given the Actions dropdown is open with the cursor on row 2 (Checkpoints)
    When I press the bare C key
    Then the checkpoints view does not open and the cursor stays on row 2

  Scenario: Wheel left and right over the bar walk the ring
    Given the board has 3 open sessions and the last column (blocked) is focused
    When I scroll the wheel right once over the bar row
    Then the Actions item is focused
    When I scroll the wheel left once over the bar row
    Then the last column (blocked) is focused again

  Scenario: Esc with no dropdown open is unchanged
    Given no menu dropdown is open and the menu bar is not focused
    When I press Esc once
    Then the app-level exit cascade runs exactly as before the menu bar existed

  Scenario: Clicking a chip activates that session
    Given the board has 2 open sessions and is focused on a kanban column
    When I click chip #1 in the bar
    Then the active view flips to session #1's agent view

  Scenario: Clicking the already open item closes the dropdown
    Given the Actions dropdown is open and the Actions item is focused
    When I click the Actions menu item again
    Then the dropdown closes and the Actions item stays focused

  Scenario: Wheel up and down over an open dropdown move the cursor
    Given the Help dropdown is open with the cursor on row 1 (Exit)
    When I scroll the wheel down once over the dropdown
    Then the cursor wraps back to row 0 (Help)

  Scenario: Clicking outside the dropdown closes it while the click still works
    Given the Help dropdown is open and the board is focused on a kanban column with a card selected
    When I click a work-unit card in another column
    Then the dropdown closes and that card becomes selected in its column
