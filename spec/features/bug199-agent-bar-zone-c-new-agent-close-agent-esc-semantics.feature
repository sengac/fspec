@done
@bug-199
@BUG-199
@bug
@menu-bar
@tui-component
@agent-view
Feature: BUG-199 — agent-bar Zone C button semantics (New Agent starts a new agent; Close Agent is the Esc exit gesture)
  """
  Regression from MENU-009 (menubar-zone-c-right-aligned-new-agent-close-agent-buttons):
  the agent bar's right-aligned Zone C buttons do not do what their labels
  say. 'New Agent' re-enters the CURRENT session (the MENU-009 R8
  current-session target substitution) instead of starting a new agent the
  way the board bar's 'New Agent' button and the '.' key do (the
  CreateSessionDialog). 'Close Agent' destroys the session immediately
  (AgentExitChoice{CloseSession} — detach, destroy_session, BackToBoard)
  instead of showing what pressing Esc in the Agent view shows (the
  RPC-098 exit cascade: interrupt when running, clear when the input has
  a draft, otherwise the 'Exit Session?' Detach / Close Session / Cancel
  confirmation dialog). And the bar gives no hint that Esc is the same
  exit gesture.

  Fix:
  1. R1 — 'New Agent' mounts the CreateSessionDialog as an overlay and the
  user stays on the Agent view until confirmation (RPC-097 reopen #1
  contract). The MENU-009 R8 current-session substitution is removed
  for the agent surface; the board surface is unchanged.
  2. R2 — 'Close Agent' runs the AgentEscPressed cascade — the EXACT
  behavior of pressing Esc in the Agent view. No session is destroyed
  without a dialog commit.
  3. R3 — the agent bar's second button label is 'Close Agent [esc]'
  (5-cell hint), so the user knows how to escape the session without
  clicking the button. The board keeps its single 'New Agent' button.
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
  #   2. R2: The agent bar's Zone C 'Close Agent' button runs the
  #      AgentEscPressed cascade (the button IS the Esc gesture): running /
  #      compacting session → interrupt the run and stay on the Agent view;
  #      non-empty input draft → clear the draft and stay; otherwise → push
  #      the 'Exit Session?' ExitConfirmationDialog (idempotent on the
  #      dialog id). The dialog's options commit through the existing
  #      AgentExitChoice bus action (Detach / Close Session / Cancel).
  #   3. R3: The agent bar's second Zone C button label is 'Close Agent
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
  #   2. Agent view on a RUNNING session: click 'Close Agent [esc]' → the
  #      run is interrupted, the user stays on the Agent view (EXACTLY the
  #      first Esc behavior) — no confirmation dialog, no teardown.
  #   3. Agent view on an idle session with a typed draft: click 'Close
  #      Agent [esc]' → the draft is cleared (EXACTLY the L6 Esc behavior)
  #      and no dialog appears.
  #   4. Agent view on an idle session with an empty input: click 'Close
  #      Agent [esc]' → the 'Exit Session?' three-button dialog (Detach /
  #      Close Session / Cancel) appears — the session is destroyed only if
  #      'Close Session' is committed.
  #   5. The agent bar at 120 cols: the row ends ' New Agent  Close Agent
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
    I want to start a new agent from the bar's 'New Agent' button and exit the session from the bar's 'Close Agent' button
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
  Scenario: 'Close Agent' on a running session interrupts the run like the first Esc
    Given the agent view shows 1 open session that is RUNNING
    And the agent bar is painted
    When I left-click the 'Close Agent [esc]' button
    Then the run is interrupted and the view stays the Agent view
    And no exit confirmation dialog is shown and the session is not destroyed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' with a non-empty input clears the draft like Esc level 6
    Given the agent view shows 1 open idle session
    And the input buffer contains a draft
    And the agent bar is painted
    When I left-click the 'Close Agent [esc]' button
    Then the input buffer is cleared and the view stays the Agent view
    And no exit confirmation dialog is shown

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' on an idle session shows the same exit confirmation as Esc
    Given the agent view shows 1 open idle session with an empty input
    And the agent bar is painted
    When I left-click the 'Close Agent [esc]' button
    Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @menu-bar-paint
  @regression
  Scenario: The agent bar paints 'New Agent' then 'Close Agent [esc]' right-aligned
    Given an agent menu bar snapshot with 0 open sessions and no ring focus
    When the agent bar is painted into a 120-column row
    Then the row ends with 'New Agent' followed by 'Close Agent [esc]' right-aligned
    And the left-anchored 'Board View' item is unchanged

  @menu-bar-paint
  @regression
  Scenario: The board bar still paints a single 'New Agent' button
    Given a board menu bar snapshot with 1 open session and no ring focus
    When the bar is painted into a 120-column row
    Then the row ends with the right-aligned 'New Agent' Zone C button
    And the row contains no 'Close Agent' button
