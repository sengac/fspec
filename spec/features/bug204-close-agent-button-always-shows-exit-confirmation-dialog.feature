@done
@bug
@bug-204
@menu-bar
@agent-view
@tui-component
@regression
Feature: BUG-204 — agent-bar 'Close' button always shows the exit confirmation dialog
  """
  Regression from BUG-199 R2 (menubar-zone-c-right-aligned-new-agent-close-agent-buttons +
  bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics): the agent bar's
  'Close' button was wired to the EXACT AgentEscPressed cascade, so its
  behavior was state-dependent:
  - running session  → interrupt the run (no dialog, no teardown)
  - non-empty draft  → clear the input buffer (no dialog, no teardown)
  - idle, empty input → push the 'Close Agent?' dialog
  This made the button's label ("Close") misleading: the user expected
  a close-action confirmation but instead got the bare Esc semantics.

  Fix: the 'Close' button ALWAYS mounts the 'Close Agent?'
  ExitConfirmationDialog (RPC-098 L7) when a session is open — it does NOT
  run the AgentEscPressed cascade. The dialog's is_busy flag is computed
  from the current session's status (Running/Compacting → the "This agent is
  working. Closing stops it and returns you to the board." variant). The dialog's options
  commit through the existing AgentExitChoice bus action (Close /
  Cancel; BUG-205 removed Detach). Pressing the physical Esc key in the Agent view is
  UNCHANGED: the full cascade (running → interrupt, draft → clear, else the
  dialog) still applies to Esc; only the button's semantics change.

  Supersedes BUG-199 R2 scenarios:
  - "Close on a running session interrupts the run like the first Esc"
  → "Close on a running session shows the exit confirmation dialog (busy variant)"
  - "Close with a non-empty input clears the draft like Esc level 6"
  → "Close with a non-empty input shows the exit confirmation dialog (draft preserved)"
  - "Close on an idle session shows the same exit confirmation as Esc"
  → unchanged (the dialog appears in all three states now)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The agent bar's 'Close' button ALWAYS mounts the
  #      'Close Agent?' ExitConfirmationDialog (RPC-098) when a session is
  #      open. It does NOT run the AgentEscPressed cascade: no interrupt
  #      branch, no draft-clear branch, no BackToBoard fallback. The
  #      dialog's is_busy flag is computed from the current session's
  #      status (Running/Compacting → busy variant).
  #   2. R2: The dialog's options commit through the existing
  #      AgentExitChoice bus action (BUG-205 removed Detach): Close
  #      Session → full teardown (board detach, open-session
  #      removal, work-unit pointer clear, backend.destroy_session,
  #      BackToBoard); Cancel → no-op (dialog removed, stay on Agent view).
  #   3. R3: With no open session, 'Close' is a silent no-op (the
  #      dialog is meaningless without a session to exit) — NOT the Esc
  #      cascade's BackToBoard fallback.
  #   4. R4: Activating the button (Enter on the ring focus or left click)
  #      clears the agent bar's ring focus (the 'leave the bar' gesture
  #      parity with New Agent).
  #   5. R5: This supersedes BUG-199 R2. Pressing the physical Esc key in
  #      the Agent view is UNCHANGED: the full cascade (running → interrupt,
  #      draft → clear, else the same exit dialog) still applies to Esc;
  #      only the button's semantics change.
  #
  # EXAMPLES:
  #   1. Agent view on a RUNNING session: click 'Close' → the
  #      'Close Agent?' dialog appears with the running-variant
  #      description ('This agent is working. Closing stops it and
  #      returns you to the board.') — the run is NOT interrupted and no draft is cleared;
  #      picking 'Close' then destroys the running session and
  #      returns to the Board.
  #   2. Agent view on an idle session with a typed draft in the input:
  #      click 'Close' → the 'Close Agent?' dialog appears (idle
  #      variant) and the typed draft is still in the input — the
  #      draft-clear Esc branch never ran.
  #   3. Agent view on a RUNNING session: press the physical Esc key →
  #      the run is interrupted (the existing Esc cascade is unchanged by
  #      this fix); only the 'Close' button no longer triggers that
  #      branch.
  #   4. Agent view on an idle session with an empty input: click
  #      'Close' → the 'Close Agent?' dialog appears; picking
  #      'Close' destroys the session and returns to the Board;
  #      picking 'Cancel' closes the dialog and stays on the Agent view.
  #
  # ========================================
  Background: User Story
    As a TUI user in the Agent view
    I want to close the session from the bar's 'Close' button
    So that the button actually closes the agent (gated by the exit confirmation) instead of behaving like an Esc keypress that only interrupts the run or clears the draft

  # ========================================
  # SCENARIOS
  # ========================================
  @mouse-events
  @regression
  Scenario: 'Close' on a running session shows the exit confirmation dialog (busy variant)
    Given the agent view shows 1 open session that is RUNNING
    And the agent bar is painted
    When I left-click the 'Close' button
    Then the 'Close Agent?' confirmation dialog (Close / Cancel) is shown
    And the dialog shows the running-variant description 'This agent is working. Closing stops it and returns you to the board.'
    And the run is NOT interrupted and the session is not destroyed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close' with a non-empty input shows the exit confirmation dialog (draft preserved)
    Given the agent view shows 1 open idle session
    And the input buffer contains a draft
    And the agent bar is painted
    When I left-click the 'Close' button
    Then the 'Close Agent?' confirmation dialog (Close / Cancel) is shown
    And the dialog shows the idle-variant description 'Closing ends this agent and returns you to the board.'
    And the input buffer still contains the draft (the draft-clear Esc branch did NOT run)
    And the session is not destroyed

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close' on an idle session shows the exit confirmation dialog (unchanged)
    Given the agent view shows 1 open idle session with an empty input
    And the agent bar is painted
    When I left-click the 'Close' button
    Then the 'Close Agent?' confirmation dialog (Close / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @regression
  Scenario: Enter on the agent's 'Close' button shows the exit confirmation dialog
    Given the agent view shows 1 open idle session and the agent bar is painted
    When the ring focuses 'Close' and I press Enter
    Then the 'Close Agent?' confirmation dialog (Close / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: '[ Close ]' then the dialog's 'Close' destroys the session and returns to Board
    Given the agent view shows 1 open session attached to work unit AUTH-001
    And the agent bar is painted
    And I left-click the 'Close' button
    When I commit 'Close' in the 'Close Agent?' dialog
    Then the session is destroyed (backend.destroy_session)
    And the view returns to the Board view

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: '[ Close ]' then Enter commits the pre-selected 'Close'
    Given the agent view shows 1 open session and the agent bar is painted
    And I left-click the 'Close' button
    When I press Enter (Close is pre-selected)
    Then the session is destroyed (destroy_session called)
    And the view returns to the Board view

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close' commits 'Cancel' to stay on the Agent view
    Given the agent view shows 1 open idle session and the agent bar is painted
    And I left-click the 'Close' button
    When I commit 'Cancel' in the 'Close Agent?' dialog
    Then the dialog is dismissed
    And the view stays the Agent view
    And the session is not destroyed

  @keyboard-navigation
  @regression
  Scenario: Pressing Esc on a running session still interrupts the run (Esc cascade unchanged)
    Given the agent view shows 1 open session that is RUNNING
    When I press Esc in the Agent view
    Then the run is interrupted
    And the view stays the Agent view
    And no exit confirmation dialog is shown

  @keyboard-navigation
  @regression
  Scenario: Pressing Esc with a non-empty draft still clears the draft (Esc cascade unchanged)
    Given the agent view shows 1 open idle session
    And the input buffer contains a draft
    When I press Esc in the Agent view
    Then the input buffer is cleared
    And the view stays the Agent view
    And no exit confirmation dialog is shown
