@done
@bug
@menu-bar
@mux
@keyboard-navigation
@tui-component
@bug-201
@BUG-201
Feature: Mux board-pane ring seams fire off the board pane + stale board bar state survives /mux on
  """
  Fix: (1) the two seam crossings in menu_move_mux (app/dispatch_menu_mux.rs) are gated on App::mux_board_pane_focused() — with the Board pane NOT focused the mux bar ring is the closed items⇄Zone B loop MenuSnapshot::advance already computes (no column stops), and the board_store's focused_column is left untouched. (2) every mux entry path (handle_mux_on, /mux default tail, apply_mux_config_draft OFF→ON) clears the board store's own bar state via BoardStore::dismiss_menu() (menu_focus + open_menu reset; focused_column untouched) — the board pane's bar is suppressed in the grid (MENU-004 R-SUPPRESS), so its ring state must never survive entry. No new Action variant; the board store's ring math (MENU-002 R8) is unchanged. Regression guards: the BUG-200 board-pane-focused seam tests must stay green.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the Board pane is NOT the focused mux pane, the mux menu bar ring is a closed items⇄Zone B (view labels + chips) loop with no column stops: Right off the last Zone B cell wraps to Item(0) and Left off Item(0) wraps to the last Zone B cell. The board_store's focused_column must not be mutated by any seam crossing in this state.
  #   2. When the Board pane IS the focused mux pane, the BUG-200 seam crossings remain in effect: Right off the last Zone B cell lands on the first column (Backlog) and dismisses the bar; Left off Item(0) lands on the last column (Blocked) and dismisses the bar.
  #   3. Entering mux mode (via /mux on, /mux default, or committing the mux config dialog) clears the board store's bar state — menu_focus and open_menu are reset — so Left/Right in the board pane walks the visible columns, never the board's invisible (suppressed) bar. The board's focused_column is NOT reset.
  #
  # EXAMPLES:
  #   1. Mux [Board|Agent] with the AGENT pane focused; the bar is engaged at the last chip (entered via empty-input Left on the agent pane). Pressing Right wraps to Item(0) (Kanban) and the board's focused_column is unchanged — the bar behaves as a closed loop.
  #   2. In mux with the board on the left and an agent on the right, the user's highlight is on the agent pane. They navigate the top menu bar and keep pressing Right past the last session chip: the highlight returns to the first menu item (Kanban) — it does NOT jump to the board's first column, and the board columns are not touched.
  #
  # ========================================
  Background: User Story
    As a TUI user (mux mode)
    I want to walk the mux menu-bar ring while a pane other than the Board pane is focused, and enter mux mode from a Board view that had its own bar engaged
    So that the ring behaves as a closed items⇄chips loop (never crossing into the board columns off the board pane) and the board's invisible bar state is dropped on entry, so Left/Right never walk a dead bar

  Scenario: Right from the last chip with the agent pane focused wraps to the first menu item
    Given the mux grid is [Board | Agent] with 1 open session and the AGENT pane is focused (the bar was engaged at the first menu item via the empty-input Left entry rule)
    When I walk the ring Right through the menu items and the view labels onto the last session chip
    Then the highlight wraps to the first menu item (Kanban) in the top bar, the bar stays engaged, and the board's focused column is unchanged (the board pane is not focused, so the ring is the closed items⇄chips loop and no column stop is entered)
    When I walk the ring Left from the first menu item
    And the highlight wraps to the last session chip (the ring closes on both ends without any column stop)

  Scenario: Left from the first menu item with the agent pane focused wraps to the last chip
    Given the mux grid is [Board | Agent] with 1 open session and the AGENT pane is focused with an empty input, and the agent pane's empty-input Left entry rule has engaged the bar at the first menu item (Kanban)
    When I press Left once
    Then the highlight wraps to the last session chip in the top bar (not onto the board's last column), the bar stays engaged, and the board's focused column is unchanged

  Scenario: Entering mux via /mux on from a Board view with an engaged bar clears the board bar state
    Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the ring was walked Right off the last column (blocked) so the board store's ring focus sits on Item(0) and its focused column is blocked
    When I submit the slash command "/mux on"
    Then mux mode is active and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)
    When I press Left once (the Board pane is the focused mux pane, the board's fresh entry focus)
    Then the board's focused column moves to the second-to-last column (done) — the Left walks the visible columns, never the board's invisible (suppressed) bar

  Scenario: Committing the mux config dialog from off to on clears the board bar state
    Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the board store's ring focus sits on Item(0) and its focused column is blocked
    When I open the mux config dialog ("/mux"), set the Enabled row to On and commit it (the dialog emits MuxConfigApplied with the enabled draft)
    Then mux mode is active and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)

  Scenario: Entering mux via /mux default clears the board bar state
    Given the single Board view is showing with the board's own bar engaged at the first menu item (Kanban) — the board store's ring focus sits on Item(0) and its focused column is blocked
    When I submit the slash command "/mux default"
    Then mux mode is active with the default preset and the board store's bar state is cleared — no ring focus and no open dropdown on the board store — while the board's focused column stays on blocked (NOT reset by the entry)

  Scenario: With the board pane focused the BUG-200 seam crossings stay in effect
    Given the mux grid is [Board | Agent] with 1 open session and the Board pane is focused with the ring on the last session chip
    When I press Right once
    Then the highlight clears from the top bar and the board's first column (backlog) is focused (the seam crossing stays in effect on the board pane)
