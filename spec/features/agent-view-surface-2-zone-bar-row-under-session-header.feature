@done
@menu-bar
@tui-component
@keyboard-navigation
@agent-view
@MENU-003
Feature: Agent view surface — 2-zone bar row under SessionHeader (menu_row flag) + empty-input left-arrow entry with GUI dropdown tracking
  """
  Layout seam: PaneSession gains `menu_row: bool`; `AgentView::pane_layout_constraints` (views/agent.rs L270-278, pinned by rpc013-source-shape) returns the 6-constraint list [Length(1), Length(1), Length(role_height), Min(0), Length(1), Length(input_height)] ONLY when the flag is set; flag-off stays the pinned 5-list byte-identical. ChromeAreas gains `menu: Rect` (height 0 when off); paint_menu_bar no-ops on zero height (same guard as every other painter).
  Dispatch ripple (MENU-002 reuse): App::dispatch_menu's MenuChipActivate arm must be context-aware — on the BOARD it flips to the Agent view (MENU-002); on the AGENT view (MENU-003) it navigates within the agent view to open_sessions[index] via the existing Shift+arrow navigation path (set_session_index_clamped in store/agent_view/navigation.rs). The board arm's behavior must not regress. Same Menu* Action set, no new variants in components/mod.rs.
  State: AgentView gains menu_focus: Option<MenuFocus> + open_menu: Option<(usize, usize)> as plain fields (AgentView is &mut through render_session_pane — no RefCell, unlike the board's BoardView Cell) + last_menu_item_rects/last_menu_chip_rects caches refreshed by the FOCUSED pane only (BUG-163). Ring = items + chips only (no columns in the agent view).
  Key seam: AgentView::handle_event gains an early arm BEFORE the input arms — if menu_focus.is_some() handle the ring/dropdown and consume; else if input.is_empty() && bare Left (no modifiers) → menu_focus = Some(Item(0)), consume. Character keys while bar-focused (closed) clear menu_focus and return Ignored so the cascade types them into the MultiLineInput on the same event. Empty-draft check = self.input.is_empty() (same seam as the Home-key arm at views/agent/dispatch.rs L192).
  Render seam: pane_render.rs::render_session_pane paints the bar into ChromeAreas.menu AFTER the header; the dropdown is painted by the pane AFTER the scrollback/input (anchored overlay, turn-modal precedent — paint_turn_modal renders into the full pane area after the main content), clamped to the pane rect, height-clipped with the MENU-001 '⋯' rule. MENU-004 note: mux agent panes use menu_row=false so the mux-level bar (MENU-004) is the only bar on screen in mux mode.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R4 (ring): Left/Right (h/l) while bar-focused walk the unified ring: the 'Board View' item (MENU-007 supersession) then session chips, then the right-aligned Zone C buttons 'New Agent' / 'Close Agent' (MENU-009), wrapping from the last Zone C button back to Item(0); Left from Item(0) wraps to the LAST Zone C button ('Close Agent') — no columns in the agent view (unlike the board). When the bar row's width drops the Zone C buttons (MENU-009 R5) they hold no ring stops.
  #   2. R3 (entry): with the input draft EMPTY, a bare Left arrow (h) enters the bar with focus on Item(0) (the first menu item — Zone A is the primary zone); bare arrows on a NON-EMPTY draft stay the text cursor; Shift+Left/Right session cycling is untouched. Right arrow does NOT enter the bar (entry is Left-only).
  #   3. R2: Non-mux Zone B is chips only — one chip per open session (index = open-sessions index, 1-based display), no view labels; with zero open sessions the bar shows 'Board View' with no separator and no chips.
  #   4. R1: The agent pane gains a 1-row MenuBar strip directly BELOW the SessionHeader (pane layout = Header, MenuBar, RoleBanner, Scrollback, Footer, Input), gated by a `menu_row: bool` flag on `PaneSession`; when the flag is off the pinned 5-constraint `pane_layout_constraints` list stays byte-identical (rpc013-source-shape keeps passing). Single-view mode sets the flag; mux agent panes set it false (MENU-004 paints the mux-level bar). ChromeAreas gains `menu: Rect` (height 0 when off; painters no-op on zero height).
  #   5. R5: Enter on a focused session chip jumps to that session in single-view mode — the chip index IS the open-sessions index; resolution reuses the Shift+arrow navigation path (navigate-to-index over the agent_view store) emitted as Action::MenuChipActivate(j) and resolved in App::dispatch. New Agent stays the '.' menu entry.
  #   6. R6: While a menu dropdown is open the bar is true-modal (BUG-161 parity): Up/Down (and wheel ScrollUp/ScrollDown) move the cursor between rows wrapping at the ends; Left/Right ring-walk and open the adjacent item's dropdown; all other keys are swallowed. Enter executes the highlighted row and closes the dropdown. [MENU-007 supersession: the agent bar's single 'Board View' item has NO dropdown — the agent view is now dropdown-free; these rules govern the board/mux surfaces.]
  #   7. R7 (mouse): clicking a menu item focuses it AND opens its dropdown (clicking the already-open item closes it); clicking a chip activates that session; the bar row hit-test runs before the input/scrollback tests; wheel Left/Right over the bar walks the ring like the keys; wheel over the bar with no dropdown open is ignored; wheel up/down over the open dropdown moves the cursor. [MENU-007 supersession: the agent bar's single 'Board View' item ACTIVATES on click (returns to the Board view) instead of opening a dropdown; the chip rule is unchanged.]
  #   8. R9 (character forwarding): any printable character key pressed while the bar is focused (dropdown closed) clears the bar focus AND is forwarded to the input on the same event (typed) — handle_event returns Ignored after clearing focus so the dispatch cascade reaches the MultiLineInput. GUI parity: typing dismisses the menu.
  #   9. R10 (Esc two-stage): with a dropdown open, Esc closes the dropdown (item focus remains); with the dropdown closed but bar-focused, Esc clears bar focus only (input regains focus; this is the ONE place Esc does NOT mean 'back to board' while bar-focused). With the bar unfocused, Esc is unchanged (App-level cascade). Up/Down on a closed bar focus drops focus back into the input.
  #   10. R8 (state): AgentView gains menu_focus: Option<MenuFocus> + open_menu: Option<(usize, usize)> as plain fields (AgentView is &mut through render_session_pane — no RefCell needed, unlike the board) plus last_menu_item_rects / last_menu_chip_rects caches refreshed by the FOCUSED pane only (BUG-163 rule: only the focused pane refreshes view-level geometry caches).
  #   11. Gate the new row behind a `menu_row: bool` flag on `PaneSession`. Single-view sets flag-on (6-constraint list, `menu` Rect between header and role); mux agent panes set flag-off (5-constraint list byte-identical — rpc013-source-shape keeps passing). ChromeAreas gains `menu: Rect` (height 0 when off; painters no-op on zero height). This matches how PaneSession already carries per-pane selection (BUG-163) and keeps every existing non-opting-in test stable.
  #   12. Bare Left ONLY (when the input draft is empty) enters the bar; bare Right stays the text cursor in every state. Left-only entry avoids the draft being silently cleared and avoids a surprise ring jump when the user is reaching for the text cursor at the end of a line.
  #   13. Forward the character to the input on the same event (typed) after clearing bar focus — GUI menus dismiss on typing; the keystroke is not lost.
  #
  # EXAMPLES:
  #   1. With zero open sessions the agent bar paints 'Board View' with no separator and no chips; Left from an empty input still enters the ring at Item(0) and Right from 'Board View' wraps to itself. [MENU-007]
  #   2. With 2 open sessions and an empty input, pressing bare Left lands on the 'Board View' item (inverse, no dropdown — MENU-007); Right lands on chip #1; chip #2; 'New Agent'; 'Close Agent'; Right wraps back to 'Board View'. [MENU-009]
  #   3. Bar-focused on chip #2 (a Running session), pressing Enter jumps the single-view agent pane to that session (same target as Shift+Right from session 1) and clears the bar focus.
  #   4. While typing 'hel' in the input, bare Left moves the text cursor left (does NOT enter the bar); clearing the draft and pressing Left then enters the bar at Item(0).
  #   5. Bar-focused (dropdown closed), pressing 'x' clears the bar focus and types 'x' into the input on the same keystroke.
  #   6. Clicking chip '#1' jumps to session #1. [MENU-007: the old 'click Help → dropdown' example is superseded — the agent bar paints only 'Board View', which activates on click.]
  #   7. Bar-focused with dropdown closed, pressing Esc once returns focus to the input (stays in the agent view, NOT back to the board); pressing Esc again with the bar unfocused then goes back to the board as before.
  #   8. Bar-focused on 'Actions', pressing Enter opens the dropdown under the item (cursor on row 0, painting over the scrollback); pressing Enter again executes row 0 ('.' New Agent) and closes the dropdown.
  #   9. Bar-focused with the dropdown closed, pressing Up or Down drops focus back into the input (bar highlight clears, text cursor active, draft unchanged); the input row keeps its draft.
  #   10. With the dropdown open, pressing a bare 'a' (not a menu entry shortcut) is swallowed (nothing typed, nothing opened); pressing Esc closes the dropdown leaving 'Actions' still focused.
  #   11. With the dropdown open over the agent pane, pressing Right walks the ring to 'Help' and re-anchors the open dropdown under 'Help' (the cursor resets to row 0 of the newly opened dropdown).
  #
  # QUESTIONS (ANSWERED):
  #   Q: Layout strategy for the agent bar: gate the new row behind a `menu_row: bool` flag on `PaneSession` (flag-on = 6-constraint list, flag-off stays byte-identical so rpc013-source-shape keeps passing), or always insert the row and supersede the pinned 5-row shape + regenerate every agent snapshot?
  #   A: Gate it behind the `menu_row` flag — single-view sets flag-on (6-constraint list, `menu` Rect between header and role); mux agent panes set flag-off (5-constraint list byte-identical — rpc013-source-shape keeps passing). ChromeAreas gains `menu: Rect` (height 0 when off; painters no-op on zero height).
  #   Q: Entry key into the bar: only bare Left (with empty input), or both Left and Right?
  #   A: Bare Left ONLY (with empty input) enters the bar; bare Right stays the text cursor in every state.
  #   Q: Character key while the bar is focused (dropdown closed): forward the character to the input (typed on the same event) or swallow it?
  #   A: Forward it — the character is typed into the input on the same event after clearing bar focus (GUI parity: typing dismisses the menu).
  #
  # ========================================
  Background: User Story
    As a TUI user (agent view)
    I want to navigate the unified menu bar + session chips below the SessionHeader via a bare Left arrow (empty input) and the items→chips ring
    So that jump between sessions, open dialogs and switch views from the agent view without leaving the keyboard flow — the same bar experience as the board (MENU-002)
    [MENU-007 supersession: the agent bar's Zone A is a single 'Board View' item (no Actions/Help, no dropdowns) that returns to the Board view when activated; Zone B keeps the session chips.]

  Scenario: The agent pane row under the header paints the 2-zone menu bar
    Given the agent pane has 2 open sessions (one running, one idle) and an empty input
    When the agent pane renders
    Then the row directly below the SessionHeader shows the 'Board View' item a separator and the two session chips and no other chrome row shifts

  Scenario: The agent pane row is gated by the menu row flag
    Given a mux agent pane with the menu row flag off
    When the pane renders
    Then the pane layout is the pinned 5-row shape (header role scrollback footer input) and no menu bar row appears

  Scenario: Bare Left on an empty input enters the bar at the first item
    Given the agent pane has an empty input
    When I press bare Left once
    Then the 'Board View' item paints inverse-video with no dropdown open

  Scenario: Bare Left with a non-empty draft stays the text cursor
    Given the input draft is the text "hello"
    When I press bare Left once
    Then the text cursor moves left and the menu bar does not gain focus

  Scenario: Bare Right never enters the bar
    Given the agent pane has an empty input
    When I press bare Right once
    Then the menu bar does not gain focus and the text cursor position is unchanged

  Scenario: The ring walks items then chips then the Zone C buttons then wraps to the first item
    Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    When I press Right five times
    Then the focus lands on chip #1 then chip #2 then 'New Agent' then 'Close Agent' and finally back on 'Board View'

  Scenario: The ring wraps left from the first item to the last Zone C button
    Given the agent pane has 3 open sessions and the menu bar is focused on the 'Board View' item
    When I press Left once
    Then the 'Close Agent' button paints inverse-video (the last Zone C button is the ring's last stop — MENU-009)
    And pressing Left once more focuses the 'New Agent' button

  Scenario: Enter on a chip jumps to that session in the agent view
    Given the agent pane has 3 open sessions and the menu bar is focused on chip #2
    When I press Enter once
    Then the agent pane shows session #2 and the bar highlight clears

  # ── MENU-007 supersession ─────────────────────────────────────────────
  # The agent-dropdown scenarios of MENU-003 (Enter on a menu item opens
  # its dropdown / Enter on an open dropdown executes / Esc two-stage /
  # bare-char swallowed with dropdown open / Left+Right re-anchor /
  # Clicking a menu item opens its dropdown / Clicking the already open
  # item closes / Wheel up-down over an open dropdown) are REMOVED: the
  # agent bar's single 'Board View' item has no dropdown, so the agent
  # view is dropdown-free. The dropdown machinery (MENU-001 component,
  # board MENU-002, mux MENU-004) is unchanged and still covered by its
  # own feature files.
  # ─────────────────────────────────────────────────────────────────────

  Scenario: Enter on the 'Board View' item returns to the Board view
    Given the agent pane is focused on the 'Board View' menu item
    When I press Enter once
    Then the active view is the Board view and the bar highlight clears

  Scenario: A character key dismisses the bar and is typed into the input
    Given the menu bar is focused (no dropdown) and the input is empty
    When I press the bare x key
    Then the bar highlight clears and the input draft is the text "x"

  Scenario: Up and Down from a closed bar drop focus back into the input
    Given the agent pane has a draft of "hi" and the menu bar is focused on chip #1
    When I press Up once
    Then the bar highlight clears the text cursor is active in the input and the draft is unchanged

  Scenario: Esc with the bar unfocused is unchanged
    Given the menu bar is not focused and the agent pane input is focused
    When I press Esc once
    Then the app-level cascade (back to board) runs exactly as before the menu bar existed

  Scenario: Clicking 'Board View' returns to the Board view
    Given the agent pane has 2 open sessions
    When I click the 'Board View' item in the bar
    Then the active view is the Board view and no dropdown opens

  Scenario: Clicking a chip activates that session
    Given the agent pane has 2 open sessions
    When I click chip #1 in the bar
    Then the agent pane shows session #1

  Scenario: Wheel left and right over the bar walk the ring
    Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    When I scroll the wheel right once over the bar row
    Then chip #1 is focused
    When I scroll the wheel right once over the bar row
    Then chip #2 is focused

  Scenario: Wheel left and right over the bar with no dropdown and no bar focus are ignored
    Given the agent pane has an empty input and the menu bar is not focused
    When I scroll the wheel right once over the bar row
    Then the menu bar does not gain focus

  Scenario: The bar omits the separator and chips when no sessions are open
    Given the agent pane has no open sessions
    When the agent pane renders
    Then the bar row shows only the 'Board View' item with no separator and no chips

  Scenario: With no chips the ring wraps from the item to itself
    Given the agent pane has no open sessions and the menu bar is focused on the 'Board View' item
    When I press Right three times
    Then the walk passes through 'New Agent' then 'Close Agent' and the 'Board View' item is focused again (MENU-009: the Zone C buttons are the only ring stops besides the item)
