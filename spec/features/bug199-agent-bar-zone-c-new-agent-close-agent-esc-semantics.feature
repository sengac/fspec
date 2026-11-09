@done
@bug-199
@BUG-199
@bug
@menu-bar
@tui-component
@agent-view
Feature: BUG-199 — agent-bar Zone C button semantics (New Agent starts a new agent; Close is the Esc exit gesture)
  """
  Regression from MENU-009 (menubar-zone-c-right-aligned-new-agent-close-agent-buttons):
  the agent bar's right-aligned Zone C buttons do not do what their labels
  say. 'New Agent' re-enters the CURRENT session (the MENU-009 R8
  current-session target substitution) instead of starting a new agent the
  way the board bar's 'New Agent' button and the '.' key do (the
  CreateSessionDialog). 'Close' destroys the session immediately
  (AgentExitChoice{CloseSession} — detach, destroy_session, BackToBoard)
  instead of showing what pressing Esc in the Agent view shows (the
  RPC-098 exit cascade: interrupt when running, clear when the input has
  a draft, otherwise the 'Close Agent?' Close / Cancel (Detach removed by BUG-205)
  confirmation dialog). And the bar gives no hint that Esc is the same
  exit gesture.

  Fix:
  1. R1 — 'New Agent' mounts the CreateSessionDialog as an overlay and the
  user stays on the Agent view until confirmation (RPC-097 reopen #1
  contract). The MENU-009 R8 current-session substitution is removed
  for the agent surface; the board surface is unchanged.
  2. R2 — 'Close' runs the AgentEscPressed cascade — the EXACT
  behavior of pressing Esc in the Agent view. No session is destroyed
  without a dialog commit.
  3. R3 — the agent bar's second button label is 'Close [esc]'
  (5-cell hint), so the user knows how to escape the session without
  clicking the button. The board keeps its single 'New Agent' button.

  SUPERSESSION NOTE (BUG-204): R2 was superseded by BUG-204 — 'Close
  Agent' ALWAYS mounts the 'Close Agent?' dialog (no interrupt /
  no draft-clear branch); only the physical Esc key keeps the full
  cascade. The two R2 scenarios that encoded the state-dependent
  behavior ('Close' on a running session interrupts the run /
  clears the draft) were removed; the idle-session scenario remains
  valid (BUG-204 keeps the dialog for every state).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The agent bar's Zone C 'New Agent' button ALWAYS starts a new
  #      agent — it mounts the CreateSessionDialog as an overlay (idempotent
  #      on the dialog id, context-aware title from the current session's
  #      WorkUnitContext) and the user stays on the Agent view until the
  #      dialog is confirmed. The existing session is never disturbed.
  #   2. R2 (SUPERSEDED by BUG-204): The agent bar's Zone C 'Close'
  #      button ALWAYS mounts the 'Close Agent?' dialog (the running /
  #      draft variants now show the dialog too, not interrupt/clear).
  #      The physical Esc key still runs the full cascade.
  #   3. R3: The agent bar's second Zone C button label is 'Close
  #      [esc]'; the first button keeps its exact 'New Agent' label. The
  #      board surface keeps its single 'New Agent' button unchanged.
  #      Geometry: Zone C still paints right-aligned and drops before Zone
  #      B content when the row cannot afford its width (the width math
  #      includes the 5-cell hint; the R6 truncation ladder and the
  #      area-width bound are unchanged).
  #
  # EXAMPLES:
  #   1. Agent view with 2 open sessions: click 'New Agent' → the Create
  #      Session dialog mounts over the Agent view; the current session is
  #      unchanged until the dialog is confirmed.
  #   2. (SUPERSEDED by BUG-204) Agent view on a RUNNING session: click
  #      'Close' → the 'Close Agent?' dialog appears (busy variant);
  #      the run is NOT interrupted.
  #   3. (SUPERSEDED by BUG-204) Agent view on an idle session with a
  #      typed draft: click 'Close' → the 'Close Agent?' dialog
  #      appears; the draft is NOT cleared.
  #   4. Agent view on an idle session with an empty input: click 'Close
  #      Agent [esc]' → the 'Close Agent?' two-button dialog
  #      (Close / Cancel) appears — the session is destroyed only if
  #      'Close' is committed.
  #   5. The agent bar at 120 cols: the row ends ' New Agent  Close
  #      [esc]' (right-aligned, 2-cell gap); the board bar is unchanged
  #      (a single right-aligned 'New Agent').
  #   6. A 40-col board row (1 chip bound to a long WU id): the board's
  #      'New Agent' button still drops (no room) while the chip keeps
  #      painting (Zone B ladder step 1) — the ladder and the area-width
  #      bound are unchanged by the agent label.
  #
  # ========================================
  Background: User Story
    As a TUI user in the Agent view
    I want to start a new agent from the bar's 'New Agent' button and exit the session from the bar's 'Close' button
    So that the agent-bar Zone C buttons behave exactly like their board/Esc counterparts instead of re-opening the current session or destroying it without confirmation

  # ========================================
  # SCENARIOS
  # ========================================
  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: Enter on the agent's 'New Agent' button mounts the Create Session dialog
    Given the agent view shows 2 open sessions and the agent bar is painted
    When the ring focuses the agent bar's 'New Agent' and I press Enter
    Then the CreateSessionDialog mounts over the Agent view
    And the current session is unchanged and the view stays the Agent view
    And the agent bar's ring focus clears

  @mouse-events
  @regression
  Scenario: A left click on the agent's 'New Agent' button mounts the Create Session dialog
    Given the agent view shows 2 open sessions and the agent bar is painted
    When I left-click the 'New Agent' button
    Then the CreateSessionDialog mounts over the Agent view
    And the current session is unchanged and the view stays the Agent view
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  @keyboard-navigation
  @mouse-events
  @regression
  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close' on an idle session shows the same exit confirmation as Esc
    Given the agent view shows 1 open idle session with an empty input
    And the agent bar is painted
    When I left-click the 'Close' button
    Then the 'Close Agent?' confirmation dialog (Close / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @menu-bar-paint
  @regression
  Scenario: The agent bar paints '[ New Agent ]' then '[ Close ]' right-aligned
    Given an agent menu bar snapshot with 0 open sessions and no ring focus
    When the agent bar is painted into a 120-column row
    Then the row ends with '[ New Agent ]' followed by '[ Close ]' right-aligned
    And the left-anchored 'Board View' item is unchanged

  @menu-bar-paint
  @regression
  Scenario: The board bar still paints a single 'New Agent' button
    Given a board menu bar snapshot with 1 open session and no ring focus
    When the bar is painted into a 120-column row
    Then the row ends with the right-aligned '[ New Agent ]' Zone C button
    And the row contains no 'Close' button
