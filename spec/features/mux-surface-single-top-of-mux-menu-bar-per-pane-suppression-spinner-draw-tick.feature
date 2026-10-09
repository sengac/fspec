@done
@menu-bar
@mux
@keyboard-navigation
@tui-component
@MENU-004
Feature: Mux surface — single top-of-mux menu bar, per-pane suppression, spinner draw tick
  """
  State seam: MultiplexLayout gains menu_focus: Option<MenuFocus> + open_menu: Option<(usize, usize)> + cached item/chip rects + open-panel rect (plain fields, like the MENU-003 AgentView pattern; no RefCell — the mux layout is &mut through render_with_stores). New views/multiplex/menu_snapshot.rs builds the per-frame MenuSnapshot (pane labels from effective_panes() with focused-pane active flag + global chips via active_menu_session_ids/chip_inputs from views/board/menu_snapshot.rs, shared — the chip list is GLOBAL, RED CARD 3). New views/multiplex/menu_keys.rs owns the bar key arms (ring walk via MenuSnapshot::advance with zone_b = pane labels + chips; true-modal dropdown; char-key forward = clear focus + return Ignored so the focused agent pane's input types it). Render: views/multiplex/render.rs reserves the top row (bar = row 0 of area, body = area.y+1..area.height-2; guard area.height < 4 -> paint legacy layout, RED CARD 4), paints the bar after panes + flash and before dividers/footer, builds the dropdown panel over the pane area (turn-modal overlay precedent). Board suppression: views/board/render.rs render_with_store gains a menu_suppressed flag (callers: single-view = false, mux board pane = true) — suppressed path skips the paint_menu_bar call + leaves row 3 blank; geometry cache cleared so mouse arms stay inert.
  Dispatch + tick seams: App::dispatch_menu (app/dispatch_menu.rs) gains a ViewMode::Mux branch — when active_view == Mux the MenuMove/MenuOpenDropdown/MenuCloseDropdown/MenuDropdownCursor/MenuFocusToColumns/MenuMoveToItem/MenuChipActivate/MenuExecuteItem arms mutate the mux layout's bar state instead of BoardStore (existing arms unchanged for Board; Agent arm from MENU-003 unchanged). ChipActivate in mux: resolve against active_menu_session_ids, focus_session_index, NO active_view flip (stay in Mux). View-label focus is a NEW concern: MenuMoveToItem is items-only, so a new Action variant MenuFocusToPane(usize) OR reuse MenuChipActivate with a pane-index encoding — DECISION: add Action::MenuFocusPane(usize) (one new variant, small) since chips and panes need different resolution. Tick: Navigator::is_menu_bar_animating() = active_view in {Board, Agent, Mux} AND any open session status in {Running, Compacting} (agent_view_store scan); app/mod.rs tick_should_draw gains a 6th operand (existing 5-operand tests + 1 new); run_loop.rs passes it. Clock: the bar painter receives clock_ms — the mux render path reads a monotonic frame clock (SystemTime ms, same source as board render's now_ms()); tests pin the glyph with a fixed value.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R-SUPPRESS: In mux mode there is exactly ONE bar on screen — the per-pane bars are suppressed. Board pane: its header row 3 paints BLANK (a menu_suppressed flag on the board render path; the 4-row header strip still renders, row 3 just stays empty). Agent panes: PaneSession.menu_row = false (MENU-003 flag — the 5-constraint chrome is unchanged and no per-agent bar paints). Files/Checkpoints panes: unaffected (they never had a row-3 chord).
  #   2. R-STATE: In mux the menu bar's state (ring focus, open dropdown, cached item/chip hit-rects) lives on MultiplexLayout (RED CARD 1 resolved) — NOT the board store — because the bar belongs to the MUX layer, not a pane. Single-view keeps its state on BoardStore/AgentView (unchanged). The ring math is shared via parameterized helpers (board columns + items + chips vs. mux items + zone_b); App::dispatch picks the holder by navigator.active_view. Mux state is reset when mux is disabled (no stale focus/dropdown on exit).
  #   3. R-KEYS: The mux trap (focused pane only) is preserved. The bar is entered from the FOCUSED pane's edge rule: Board pane → the MENU-002 column⇄menu⇄chip ring continues into the mux bar (→ off the last column lands on the bar's first item, then chips, then wraps); Agent pane → the MENU-003 empty-input Left entry (empty draft + bare Left enters at Item(0); character key dismisses the bar and is typed into the focused pane's input; Up/Down drop back into the input). Files/Checkpoints panes do NOT feed the bar (RED CARD 2 resolved: No entry) — their Esc-close + arrow scroll stay untouched. Shift+Left/Right pane focus cycling is UNTOUCHED and never enters the bar. The bar's open dropdown is true-modal (BUG-161 parity): Up/Down move the cursor, Left/Right walk the ring + re-anchor, Enter executes + closes, Esc closes, every other key is swallowed.
  #   4. R-CHIPS: Zone B chips in mux = the GLOBAL open-session list (Cleared dropped; active_menu_session_ids parity), in open-session order — NOT the pane window (RED CARD 3 resolved). The chips' index is the global open-sessions index; chip activation (Enter/click) focuses that session's open-session slot via the same resolution as MENU-002/003 (set_session_index_clamped / focus_session_index) but does NOT flip the view out of Mux — the focused agent pane shows it if it's in the window, otherwise the window rotates (existing MUX-002 shift semantics).
  #   5. R-ZONEB: In mux, Zone B displays the pane view labels (effective_panes() order: Board/Files/Ckpts — the focused pane's label is active/bold; agent panes contribute no view label, their sessions are the chips) followed by a separator and the GLOBAL session chips (active_menu_session_ids order). View-label focus + Enter (or click) focuses that pane (set_focus). A chip's focus + Enter (or click) activates that session (focus the open-session slot; do NOT flip the view out of Mux — the agent pane showing it stays put / the window is unaffected). The bar ring in mux = items + Zone B cells (no columns).
  #   6. R-MOUSE: The mux bar row is hit-tested BEFORE the pane/mux dividers. Clicking a menu item focuses it and toggles its dropdown (clicking the already-open item closes it); clicking a chip activates that session AND focuses the agent pane that renders it (rotating the agent window when the session is not currently in a slot — R-CHIPS) so the chip's selected-highlight (MENU-010) paints and the next event's focus-sync cannot revert the switch; clicking a view label focuses that pane. Clicking OUTSIDE the open dropdown (on a pane) closes the dropdown AND still lands the click on that pane (click-to-focus runs on the same event). Wheel ScrollLeft/ScrollRight over the bar walks the ring; wheel ScrollUp/ScrollDown over an open dropdown moves the cursor; wheel over the bar with no focus and no open dropdown is ignored. A bar click never changes pane focus via the PANE-LABEL path (the bar is an overlay control, not a pane) — the chip-activation focus move (above) is the one deliberate exception, required to make the selection visible.
  #   7. R-EXEC: Executing a dropdown row (Enter or click) resolves via the shared MenuAction registry and re-dispatches onto the bus — the SAME Action variants the board/agent surfaces use; in mux the executed action flows through the existing App routing unchanged (e.g. 'C Checkpoints' opens the checkpoints view exactly as pressing C in the board pane would; 'M Mux' opens the mux config dialog; '? Help' opens help). The mux bar's own focus/dropdown state closes on execute. NewAgent carries no session payload (live current-session snapshot substituted at execute time, MENU-002/003 parity).
  # NOTE (BUG-203): the 'New Agent' R8 substitution described here is SUPERSEDED
  # for the BOARD surface — the registry mapping is now payload-free (MenuAction
  # NewAgent → Action::OpenCreateSessionDialog { preselect: None }); the board
  # 'New Agent' gesture ALWAYS mounts the CreateSessionDialog and never resumes
  # a session. Shift+Right keeps the cycle/resume semantics. See
  # spec/features/board-new-agent-gesture-prompts-create-session-dialog.feature
  #   8. R-TICK: The 16ms draw tick gains a 'menu bar animating' operand so the Running/Compacting chip glyphs keep advancing: Navigator::is_menu_bar_animating() is true iff the ACTIVE surface paints a menu bar (Board, Agent or Mux view) AND any open session is Running or Compacting. The run loop ORs it into tick_should_draw (new 6th operand). The mux bar painter takes clock_ms from the frame clock and renders the braille frame via the chip builder (pure — tests pin the glyph with a fixed clock_ms). Single-view board/agent bars already redraw on input events; this gate is what keeps the MUX bar alive when the user sits still (the BUG-194 class of frozen spinner).
  #   9. R-LAYOUT: mux render reserves the TOP row for the menu bar (body = area.y+1 .. area.height-2, reserving the top bar row + the bottom footer row); min height rises 3 -> 4. Panes + dividers paint within the shrunk body; the bar is painted into the top row AFTER the focus flash and BEFORE dividers/footer (full width). Zone A = CATEGORIES (Actions, Help); Zone B (mux) = pane view labels in effective_panes() order (focused pane bold/active) + the GLOBAL session chips. The painter takes clock_ms for the braille frame glyph.
  #
  # EXAMPLES:
  #   1. In a mux grid [Board | Agent | Agent] with two open sessions (one running, one idle), the top row of the screen shows the menu bar 'Actions Help' followed by a separator, the pane label 'Board' (bold because it is focused), and the two session chips '#1' (animated braille dot, running) and '#2' (idle dot). No other row on the screen repeats the menu bar — each pane's own top chrome has no menu row.
  #   2. Board pane focused, cursor on the last (Blocked) column: pressing Right moves the highlight off the columns and onto the 'Kanban' menu item in the top bar; Right again lands on 'Help', then on the 'Board' pane label, then on chip #1; Right from the last chip wraps back to the FIRST column (backlog) — the ring's right edge re-enters the columns (BUG-200: one continuous columns⇄items⇄chips ring); Left from 'Kanban' wraps to the LAST column (blocked) — the ring's left edge (BUG-200).
  #   3. Agent pane focused with an empty input: pressing bare Left puts the highlight on the 'Actions' item in the top bar; typing 'x' while the bar is focused dismisses the highlight AND types 'x' into that pane's input on the same keystroke; pressing Up or Down while the bar is focused (dropdown closed) drops the highlight back into the input (the draft is kept).
  #   4. Files pane focused: pressing Left or Right scrolls the file list as before and never moves the highlight onto the top menu bar; Esc still closes the Files pane. The bar is still reachable by mouse-clicking an item or chip.
  #   5. Highlight on 'Actions' in the top bar (dropdown closed): Enter opens the dropdown at row 0 ('. New Agent'), the panel painting over the panes below. Down moves the cursor row-by-row, wrapping at the last row. Right (while the dropdown is open) walks the highlight to 'Help' and re-anchors the dropdown under 'Help' at row 0. Enter on the highlighted row executes it ('C Checkpoints' opens the checkpoints view) and closes the dropdown. Esc closes the dropdown and 'Actions' stays highlighted. Pressing 'a' while the dropdown is open is swallowed (nothing typed, nothing opened).
  #   6. Clicking the 'Help' menu item in the top bar focuses it and opens its dropdown (clicking it again closes the dropdown). Clicking chip '#2' focuses session #2 (the agent pane showing it becomes the live composer if one shows it; no view flip out of mux). Clicking the 'Files' pane label focuses the Files pane. Clicking a pane outside any open dropdown closes the dropdown AND lands the click on that pane (it gets focused). Wheel left/right over the bar walks the ring like the keys; wheel up/down over an open dropdown moves the cursor; wheel over the bar with no bar focus and no open dropdown is ignored.
  #   7. Shift+Left / Shift+Right keep cycling the FOCUSED PANE exactly as before (and rotating the agent window at the edges) — they never move the menu-bar highlight, even when the bar is focused or a dropdown is open.
  #   8. While a session is Running in mux mode, that chip's braille dot keeps advancing every 80ms even when the user presses nothing — the draw tick stays open because a menu bar is painted and a session is Running/Compacting. When every session is Idle (or the view flips to a surface without a bar), the tick gate closes and the glyph sits still.
  #   9. Mux grid [Board | Files | Ckpts] with NO open sessions: the top row shows 'Actions Help' followed by a separator and the pane labels 'Board Files Ckpts' (focused pane bold) — no chips, no extra separator run. The ring walks items → pane labels → wrap; Enter/click on a pane label focuses that pane; with zero chips there is nothing to activate.
  #   10. On a 3-row terminal (too short for bar + body + footer) the top bar does not paint at all and the mux renders exactly as before the feature existed (old body height, same panes) — graceful degradation, no crash, no regression of the existing 3-row mux behavior.
  #
  # QUESTIONS (ANSWERED):
  #   Q: In mux, where should the menu bar's focus/dropdown state live — MultiplexLayout (bar belongs to the mux layer) or reuse BoardStore?
  #   A: No — the bar is entered only from the Board pane (column ring) and the Agent pane (empty-input Left). Files/Checkpoints keep their Esc-close + arrow scroll; the bar stays mouse-clickable.
  #
  # ========================================
  Background: User Story
    As a TUI user (mux mode)
    I want to navigate ONE full-width 2-zone menu bar at the top of the mux view (Zone A menu items + Zone B pane labels with global session chips), entered from the focused Board/Agent pane's edge rule
    So that the menu bar is shared across the whole mux grid (no per-pane bars) and the Running/Compacting chip glyphs animate via a dedicated 16ms draw tick

  # ── R-LAYOUT: the bar paints at the top ─────────────────────────────
  Scenario: The mux top row paints the 2-zone menu bar full width
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions (one Running, one Idle)
    When the mux view renders
    Then the top row shows the menu items (Actions Help) a separator the pane label 'Board' and the two session chips (#1 with the running glyph #2 with the idle glyph)
    And the panes paint within the body below the bar row (each pane body starts one row lower than before the bar existed)
    And the mux footer still occupies the bottom row

  Scenario: The focused pane label is active and the chips use the global session list
    Given the mux grid is [Board | Agent | Agent] with 3 open sessions (the focused pane is the Board)
    When the mux view renders
    Then the top row shows 'Board' active and the chips '#1' '#2' '#3' in global open-session order (the window is irrelevant — chips are global)
    And Cleared sessions produce no chip

  Scenario: The bar omits the separator and chips when no sessions are open
    Given the mux grid is [Board | Files | Ckpts] with no open sessions
    When the mux view renders
    Then the top row shows 'Actions Help' followed by the pane labels 'Board Files Ckpts' (focused pane bold)
    And no chips and no extra separator run appear

  # ── R-SUPPRESS: per-pane bars are suppressed ────────────────────────
  Scenario: The board pane's header row 3 is blank in mux
    Given the mux grid is [Board | Agent] with 1 open session
    When the mux view renders
    Then the board pane's 4-row header strip still renders (logo checkpoint status divider) but its row 3 is blank (no per-pane menu bar)
    And exactly ONE menu bar is on screen (the mux top row)

  Scenario: The agent panes paint no per-pane menu bar in mux
    Given the mux grid is [Agent | Agent] with 2 open sessions
    When the mux view renders
    Then each agent pane paints its 5-row chrome (header role scrollback footer input) with no menu bar row
    And the only menu bar on screen is the mux top row

  # ── R-KEYS: entry from the focused pane's edge rule ─────────────────
  Scenario: The board column ring continues into the mux bar
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column
    When I press Right
    Then the highlight lands on the 'Actions' menu item in the top bar (leaving the columns)
    When I press Right
    Then the highlight lands on 'Help'
    When I press Right
    Then the highlight lands on the 'Board' pane label
    When I press Right
    Then the highlight lands on chip #1

  Scenario: The mux bar ring wraps in both directions
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column
    When I walk the ring Right until it wraps past the last chip
    Then the highlight returns to the first column (backlog) (the ring's right edge re-enters the columns — BUG-200)
    When I walk the ring Right across the 7 columns
    Then the highlight lands on 'Kanban' (the ring's left edge, the pre-existing entry rule)
    When I press Left from 'Kanban'
    Then the highlight wraps to the last column (blocked) (the ring's left edge — BUG-200)

  Scenario: The agent pane's empty-input Left enters the mux bar
    Given the mux grid is [Agent | Agent] with 2 open sessions and the Agent pane is focused with an empty input
    When I press bare Left
    Then the highlight lands on the 'Actions' menu item in the top bar

  Scenario: A character key while the bar is focused dismisses it and types into the focused pane input
    Given the mux grid is [Agent | Agent] with 2 open sessions the Agent pane is focused and the bar is focused (dropdown closed)
    When I type 'x'
    Then the bar highlight clears AND 'x' is typed into that pane's input on the same keystroke

  Scenario: Up and Down from a closed bar drop focus back into the focused pane input
    Given the mux grid is [Agent | Agent] with 2 open sessions the Agent pane is focused and the bar is focused (dropdown closed) with a draft 'abc' in its input
    When I press Up
    Then the bar highlight clears and the pane input keeps its draft 'abc' (cursor active)

  Scenario: The Files pane does not feed the mux bar
    Given the mux grid is [Board | Files] and the Files pane is focused
    When I press Left or Right
    Then the file list scrolls as before and the highlight never moves onto the top menu bar
    When I press Esc
    Then the Files pane closes (the bar is untouched and stays reachable by mouse)

  # ── R-KEYS/R-EXEC: the dropdown is true-modal ───────────────────────
  Scenario: Enter on a menu item opens its dropdown at row 0
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions the Board pane is focused and the 'Actions' item has the bar highlight
    When I press Enter
    Then the dropdown opens under 'Actions' with the cursor on row 0 ('. New Agent') painting over the panes below

  Scenario: Up and Down move the dropdown cursor wrapping at the ends
    Given the mux 'Actions' dropdown is open with the cursor on row 0
    When I press Down
    Then the cursor moves to row 1 ('/ Search')
    When I press Down past the last row
    Then the cursor wraps to row 0

  Scenario: While a dropdown is open Left and Right walk the ring and re-anchor
    Given the mux 'Actions' dropdown is open (cursor row 0) and 'Actions' has the bar highlight
    When I press Right
    Then the highlight moves to 'Help' and the open dropdown re-anchors under 'Help' at row 0

  Scenario: Enter on an open dropdown executes the highlighted row and closes the dropdown
    Given the mux 'Actions' dropdown is open with the cursor on the 'C Checkpoints' row
    When I press Enter
    Then the checkpoints action is dispatched (the existing App routing runs unchanged) and the dropdown closes

  Scenario: Esc closes the open dropdown and keeps the item focused
    Given the mux 'Actions' dropdown is open and 'Actions' has the bar highlight
    When I press Esc
    Then the dropdown closes and 'Actions' stays highlighted

  Scenario: While a dropdown is open all other keys are swallowed
    Given the mux 'Actions' dropdown is open
    When I press 'a' (not a menu entry shortcut)
    Then nothing is typed and nothing is opened (the key is swallowed — true-modal, BUG-161 parity)

  # ── R-CHIPS/R-ZONEB: chip + view-label activation ───────────────────
  Scenario: Enter on a chip focuses that session without flipping out of mux
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the chip '#2' has the bar highlight
    When I press Enter
    Then session #2 is focused in the agent store and the view stays in Mux (no single-view flip)

  Scenario: Enter on a pane view label focuses that pane
    Given the mux grid is [Board | Files | Ckpts] and the 'Files' pane label has the bar highlight
    When I press Enter
    Then the Files pane is focused (set_focus)

  # ── R-MOUSE ─────────────────────────────────────────────────────────
  Scenario: Clicking a menu item focuses it and opens its dropdown
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions
    When I click the 'Help' menu item in the top bar
    Then 'Help' is focused and its dropdown is open

  Scenario: Clicking the already open item closes the dropdown
    Given the mux 'Help' dropdown is open (Help item focused)
    When I click the 'Help' item again
    Then the dropdown closes

  Scenario: Clicking a chip activates that session
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions
    When I click chip '#2' in the top bar
    Then session #2 is focused and the view stays in Mux

  Scenario: Clicking a chip outside the agent window rotates the window and focuses its pane
    Given the mux grid is [Board | Agent | Agent] with 3 open sessions (the agent window shows #1 and #2) and the first Agent pane (session #1) is focused
    When I click chip '#3' in the top bar (the session is outside the 2-slot agent window)
    Then the agent window rotates so session #3 is rendered and that pane is focused
    And session #3 is the store's current session and the view stays in Mux
    And chip #3's cells carry the selected-item highlight on the next frame while chip #1's cells keep the #333333 background

  Scenario: Clicking a pane label focuses that pane
    Given the mux grid is [Board | Files | Ckpts]
    When I click the 'Ckpts' pane label in the top bar
    Then the Checkpoints pane is focused

  Scenario: Clicking outside the dropdown closes it while the click still lands on the pane
    Given the mux 'Actions' dropdown is open
    When I click the Files pane (outside the dropdown panel and off the bar items)
    Then the dropdown closes AND the Files pane is focused (the click lands on the same event)

  Scenario: Wheel left and right over the bar walk the ring
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and a bar item is focused
    When I wheel ScrollRight over the bar row
    Then the ring focus advances one stop (like the Right key)
    When I wheel ScrollLeft over the bar row
    Then the ring focus retreats one stop (like the Left key)

  Scenario: Wheel up and down over an open dropdown move the cursor
    Given the mux 'Actions' dropdown is open (cursor row 0)
    When I wheel ScrollDown over the open dropdown panel
    Then the cursor moves to row 1
    When I wheel ScrollUp over the open dropdown panel
    Then the cursor returns to row 0

  Scenario: Wheel over the bar with no bar focus and no open dropdown is ignored
    Given the mux grid is [Board | Agent | Agent] with 2 open sessions and no bar focus and no dropdown open
    When I wheel ScrollLeft or ScrollRight over the bar row
    Then nothing happens (the event is ignored — the bar is inert)

  # ── R-KEYS: Shift+arrow pane cycling is untouched ───────────────────
  Scenario: Shift+Left and Shift+Right cycle the focused pane without touching the bar
    Given the mux grid is [Board | Agent | Agent] and the Board pane is focused with the 'Actions' item highlighted in the top bar
    When I press Shift+Right
    Then the Agent pane is focused (pane focus moved) and the 'Actions' bar highlight is unchanged (Shift+arrows never enter or move the bar)

  # ── R-TICK: the draw tick keeps the chips animating ─────────────────
  Scenario: The draw tick stays open while a session is Running and a bar is painted
    Given the mux grid is [Agent | Agent] with one session Running
    When the run loop evaluates the draw-tick gate
    Then the 'menu bar animating' operand is true (active surface paints a bar AND a session is Running/Compacting)
    And the 16ms tick keeps redrawing so the Running chip's braille frame advances

  Scenario: The draw tick closes when every session is idle
    Given the mux grid is [Agent | Agent] with all sessions Idle
    When the run loop evaluates the draw-tick gate
    Then the 'menu bar animating' operand is false and the chip glyph sits still (no forced redraw)

  # ── R-LAYOUT: graceful degradation on tiny terminals ────────────────
  Scenario: A 3-row terminal does not paint the bar and keeps the legacy layout
    Given the mux grid is [Board | Agent] on a 3-row terminal (too short for bar + body + footer)
    When the mux view renders
    Then the top row does not paint a menu bar
    And the panes render with the pre-existing layout (old body height, same panes) with no crash or regression
