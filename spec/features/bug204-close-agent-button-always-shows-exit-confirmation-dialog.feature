@done
@bug
@bug-204
@menu-bar
@agent-view
@tui-component
@regression
Feature: BUG-204 — agent-bar 'Close Agent' button always shows the exit confirmation dialog
  """
  Regression from BUG-199 R2 (menubar-zone-c-right-aligned-new-agent-close-agent-buttons +
  bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics): the agent bar's
  'Close Agent' button was wired to the EXACT AgentEscPressed cascade, so its
  behavior was state-dependent:
    - running session  → interrupt the run (no dialog, no teardown)
    - non-empty draft  → clear the input buffer (no dialog, no teardown)
    - idle, empty input → push the 'Exit Session?' dialog
  This made the button's label ("Close Agent") misleading: the user expected
  a close-action confirmation but instead got the bare Esc semantics.

  Fix: the 'Close Agent' button ALWAYS mounts the 'Exit Session?'
  ExitConfirmationDialog (RPC-098 L7) when a session is open — it does NOT
  run the AgentEscPressed cascade. The dialog's is_busy flag is computed
  from the current session's status (Running/Compacting → the "The agent is
  currently running. Choose how to exit." variant). The dialog's options
  commit through the existing AgentExitChoice bus action (Detach / Close
  Session / Cancel). Pressing the physical Esc key in the Agent view is
  UNCHANGED: the full cascade (running → interrupt, draft → clear, else the
  dialog) still applies to Esc; only the button's semantics change.

  Supersedes BUG-199 R2 scenarios:
    - "Close Agent on a running session interrupts the run like the first Esc"
      → "Close Agent on a running session shows the exit confirmation dialog (busy variant)"
    - "Close Agent with a non-empty input clears the draft like Esc level 6"
      → "Close Agent with a non-empty input shows the exit confirmation dialog (draft preserved)"
    - "Close Agent on an idle session shows the same exit confirmation as Esc"
      → unchanged (the dialog appears in all three states now)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The agent bar's 'Close Agent' button ALWAYS mounts the
  #      'Exit Session?' ExitConfirmationDialog (RPC-098) when a session is
  #      open. It does NOT run the AgentEscPressed cascade: no interrupt
  #      branch, no draft-clear branch, no BackToBoard fallback. The
  #      dialog's is_busy flag is computed from the current session's
  #      status (Running/Compacting → busy variant).
  #   2. R2: The dialog's options commit through the existing
  #      AgentExitChoice bus action: Detach → BackToBoard (session keeps
  #      running); Close Session → full teardown (board detach, open-session
  #      removal, work-unit pointer clear, backend.destroy_session,
  #      BackToBoard); Cancel → no-op (dialog removed, stay on Agent view).
  #   3. R3: With no open session, 'Close Agent' is a silent no-op (the
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
  #   1. Agent view on a RUNNING session: click 'Close Agent' → the
  #      'Exit Session?' dialog appears with the running-variant
  #      description ('The agent is currently running. Choose how to
  #      exit.') — the run is NOT interrupted and no draft is cleared;
  #      picking 'Close Session' then destroys the running session and
  #      returns to the Board.
  #   2. Agent view on an idle session with a typed draft in the input:
  #      click 'Close Agent' → the 'Exit Session?' dialog appears (idle
  #      variant) and the typed draft is still in the input — the
  #      draft-clear Esc branch never ran.
  #   3. Agent view on a RUNNING session: press the physical Esc key →
  #      the run is interrupted (the existing Esc cascade is unchanged by
  #      this fix); only the 'Close Agent' button no longer triggers that
  #      branch.
  #   4. Agent view on an idle session with an empty input: click
  #      'Close Agent' → the 'Exit Session?' dialog appears; picking
  #      'Detach' returns to the Board with the session still alive;
  #      picking 'Cancel' closes the dialog and stays on the Agent view.
  #
  # ========================================

  Background: User Story
    As a TUI user in the Agent view
    I want to close the session from the bar's 'Close Agent' button
    So that the button actually closes the agent (gated by the exit confirmation) instead of behaving like an Esc keypress that only interrupts the run or clears the draft

  # ========================================
  # SCENARIOS
  # ========================================

  @mouse-events
  @regression
  Scenario: 'Close Agent' on a running session shows the exit confirmation dialog (busy variant)
    Given the agent view shows 1 open session that is RUNNING
    And the agent bar is painted
    When I left-click the 'Close Agent' button
    Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    And the dialog shows the running-variant description 'The agent is currently running. Choose how to exit.'
    And the run is NOT interrupted and the session is not destroyed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' with a non-empty input shows the exit confirmation dialog (draft preserved)
    Given the agent view shows 1 open idle session
    And the input buffer contains a draft
    And the agent bar is painted
    When I left-click the 'Close Agent' button
    Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    And the dialog shows the idle-variant description 'Choose how to exit the session.'
    And the input buffer still contains the draft (the draft-clear Esc branch did NOT run)
    And the session is not destroyed

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' on an idle session shows the exit confirmation dialog (unchanged)
    Given the agent view shows 1 open idle session with an empty input
    And the agent bar is painted
    When I left-click the 'Close Agent' button
    Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @regression
  Scenario: Enter on the agent's 'Close Agent' button shows the exit confirmation dialog
    Given the agent view shows 1 open idle session and the agent bar is painted
    When the ring focuses 'Close Agent' and I press Enter
    Then the 'Exit Session?' confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' commits 'Close Session' to destroy the session and return to Board
    Given the agent view shows 1 open session attached to work unit AUTH-001
    And the agent bar is painted
    And I left-click the 'Close Agent' button
    When I commit 'Close Session' in the 'Exit Session?' dialog
    Then the session is destroyed (backend.destroy_session)
    And the view returns to the Board view

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' commits 'Detach' to return to Board with the session still alive
    Given the agent view shows 1 open session and the agent bar is painted
    And I left-click the 'Close Agent' button
    When I commit 'Detach' in the 'Exit Session?' dialog
    Then the view returns to the Board view
    And the session is NOT destroyed (destroy_session was not called)

  @keyboard-navigation
  @mouse-events
  @regression
  Scenario: 'Close Agent' commits 'Cancel' to stay on the Agent view
    Given the agent view shows 1 open idle session and the agent bar is painted
    And I left-click the 'Close Agent' button
    When I commit 'Cancel' in the 'Exit Session?' dialog
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
