@done
@bug
@menu-bar
@mux
@keyboard-navigation
@tui-component
@bug-200
@BUG-200
Feature: Mux board-pane ring — the columns re-enter the mux bar and the bar re-enters the columns
  """
  State seam: App::dispatch_menu's Mux MenuMove arm (app/dispatch_menu.rs, body `menu_move_mux`) mirrors the board↔bar ring's two seam edges for a focused Board pane, making ONE continuous ring: col0 (backlog) … col6 (blocked) → Item0 (Kanban) … ItemN → ZoneB0 (first pane view label) … ZoneB_last (last chip) → back to col0. Each seam crossing is a single step, exactly like the single Board view's column⇄item⇄chip⇄button ring (MENU-002/009): (a) Right off col6 lands on Item0 (the pre-existing edge rule, unchanged — key path via the classify_bar_key intercept, wheel path via the mirrored dispatch); Left off Item0 lands on col6 (the board's LAST column) and clears the bar ring. (b) Right off the LAST Zone B cell (last chip when chips paint, last view-label cell when they don't) lands on col0 (backlog) and clears the bar ring; Left off col0 lands on that same last Zone B cell (the board store's own wrap into its suppressed bar is mirrored onto the MUX bar's ring, and the store's hidden bar focus is cleared via BoardStore::menu_clear_focus). Interior landings keep their own segment's walk (board store menu_move for the columns; MenuSnapshot::advance for the bar). No new Action variant.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R-COLUMNS: In Mux with the Board pane focused, the columns stay a continuous wrap-around ring half: the 7 kanban columns (col0 = backlog … col6 = blocked) walk with Left/Right exactly as in the single Board view, with the bar's two seam edges as the wrap: Right off col6 re-enters the mux bar on the first menu item (Kanban) — the pre-existing edge rule, unchanged; Left off the first menu item (Kanban) lands on col6 (blocked) with the bar ring cleared. The single Board view's Zone C 'New Agent' button has no mux analogue (MENU-009 Q1: the mux bar paints no Zone C), so the columns' right edge wraps directly onto the items.
  #   2. R-WRAP: In Mux with the Board pane focused, the bar's RIGHT edge re-enters the columns: Right from the LAST Zone B cell (the last chip when chips paint; the last view-label cell when no chips paint — the chips' absence makes the view labels the ring's last stop) lands on col0 (backlog) with the bar ring cleared. This is the fix for the reported 'right at the end of the menu bar just cycles back to the start of the menu bar' behavior: the 7 column stops are reachable from the bar side again.
  #   3. R-CHIPS: In Mux with the Board pane focused, the board ring's LEFT edge re-enters the mux bar's CHIP segment, not the items: Left from col0 wraps to the last Zone B cell (last chip / last view-label cell). The board store's own wrap into its (suppressed, invisible) bar segment is mirrored onto the MUX bar's ring, and the store's hidden bar focus clears so the next walk continues on the MUX ring. Left from an INTERIOR bar stop (an item other than Kanban, or a chip other than the last) keeps walking the bar's items⇄chips ring unchanged (no column stop until the first item — the ring is lossless: no stop swallowed, none repeated).
  #
  # EXAMPLES:
  #   1. Mux grid [Board | Agent | Agent] with 2 open sessions, Board pane focused, ring on the first menu item (Kanban): Left lands on the last column (blocked) with the bar highlight cleared; Left again walks the columns (done).
  #   2. Mux grid [Board | Agent | Agent] with 2 open sessions, Board pane focused, cursor on the first column (backlog): Left wraps to the last chip (#2, the last Zone B cell); Left again lands on chip #1; Left again on the 'Board' view label; Left again on the last menu item (Help); Left keeps walking the items (Settings, Tools) and Left from the first item wraps back to the last column (blocked) — the ring is lossless.
  #   3. Mux grid [Board | Agent | Agent] with 2 open sessions, Board pane focused, ring on the last chip (#2): Right lands on the first column (backlog) with the bar highlight cleared; Right again walks the columns (specifying).
  #   4. Mux grid [Board | Files] with no open sessions, Board pane focused, cursor on the first column (backlog): Left lands on the 'Files' pane label (the last Zone B cell — the chips' absence makes the view labels the ring's last stop).
  #
  # QUESTIONS (ANSWERED):
  #   Q: Does the single-Board "left from the first column → New Agent button" rule carry over to the mux bar?
  #   A: No — the mux bar paints no Zone C (MENU-009 Q1); the left edge wraps to the LAST Zone B cell (last chip / last view label) instead.
  #   Q: Should the whole ring (columns → items → chips → columns) be ONE continuous cycle in both directions?
  #   A: Yes (user directive 2026-10-08): right off the last column → Kanban; right off the last chip → backlog; left off the first column → last chip; left off Kanban → blocked. Every seam crossing is a single step.
  #
  # ========================================

  Background: User Story
    As a TUI user (mux mode)
    I want to cycle the board column ⇄ menu-bar item ⇄ session-chip ring with Left/Right
    So that the ring is continuous in both directions across the board pane and the mux top bar

  Scenario: Left from the first menu item lands on the last column
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the first menu item (Kanban)
    When I press Left once
    Then the highlight clears from the top bar and the board's last column (blocked) is focused
    When I press Left once
    Then the board's second-to-last column (done) is focused (the columns keep their own walk)

  Scenario: Right from the last chip lands on the first column
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the ring on the last chip (#2)
    When I press Right once
    Then the highlight clears from the top bar and the board's first column (backlog) is focused
    When I press Right once
    Then the board's second column (specifying) is focused (the columns keep their own walk)

  Scenario: Left from the first column wraps to the last chip
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the first column (backlog)
    When I press Left once
    Then the highlight lands on the last chip (#2) in the top bar (the board ring's left edge re-enters the bar's chip segment, not the menu items)
    When I press Left once
    Then the highlight lands on chip #1
    When I press Left once
    Then the highlight lands on the 'Board' pane view label (the first Zone B cell)
    When I press Left once
    Then the highlight lands on the last menu item (Help) and keeps walking the items from there (Settings then Tools on the next Lefts — the bar's items⇄chips ring has no column stop until the first item)

  Scenario: Right from the last column re-enters the bar on the first menu item
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column (blocked)
    When I press Right once
    Then the highlight lands on the first menu item (Kanban) in the top bar (the pre-existing edge rule, unchanged by BUG-200)

  Scenario: Left from the first column with no open sessions lands on the last view-label cell
    Given the mux grid is [Board | Files] with no open sessions and the Board pane is focused with the cursor on the first column (backlog)
    When I press Left once
    Then the highlight lands on the 'Files' pane label (the last Zone B cell — the chips' absence makes the view labels the ring's last stop)
