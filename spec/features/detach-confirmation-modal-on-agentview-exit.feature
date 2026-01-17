@done
@tui-component
@session-management
@TUI-046
Feature: Detach Confirmation Modal on AgentView Exit

  """
  Modifies AgentView.tsx ESC handler (Priority 5) to show ThreeButtonDialog instead of calling onExit() directly. Uses sessionDetach() and sessionManagerDestroy() from codelet-napi for session lifecycle. Reuses ThreeButtonDialog component from TUI-040 for DRY/SOLID compliance.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When user presses ESC to exit AgentView (after all other ESC priority handlers are exhausted), a confirmation modal must appear instead of immediately exiting
  #   2. The confirmation modal must use ThreeButtonDialog component for DRY/SOLID compliance (reuse existing TUI-040 pattern)
  #   3. The modal must present three options: 'Detach' (keep running in background), 'Close Session' (terminate), and 'Cancel' (stay in AgentView)
  #   4. The default selected option must be 'Detach' (index 0) since preserving work is the safest default
  #   5. Detach option must call sessionDetach(sessionId) from codelet-napi to keep the session running in background
  #   6. Close Session option must call sessionManagerDestroy(sessionId) to terminate the background session before exiting
  #   7. Cancel option must dismiss the modal and return to normal AgentView state without any session changes
  #   8. ESC key pressed while modal is showing must dismiss modal (handled by ThreeButtonDialog onCancel)
  #   9. Modal must only appear if there is an active session (sessionRef.current is not null)
  #   10. If no active session exists, ESC should exit immediately without showing modal (current behavior for new/empty state)
  #
  # EXAMPLES:
  #   1. User presses ESC with active session and empty input, confirmation modal appears with 'Detach' highlighted, user presses Enter, session detaches, view exits to kanban board
  #   2. User presses ESC with active session, modal appears, user presses Right Arrow twice to highlight 'Cancel', presses Enter, modal dismisses, user remains in AgentView
  #   3. User presses ESC with active running session (isLoading=true), modal appears, user selects 'Close Session', session is destroyed, view exits
  #   4. User presses ESC while modal is showing, modal dismisses via onCancel, user remains in AgentView with session unchanged
  #   5. User opens AgentView fresh (no session yet), presses ESC immediately, view exits without modal (no session to detach)
  #   6. User with input text presses ESC, input clears (existing Priority 4 behavior), presses ESC again, modal appears for exit confirmation
  #   7. User selects 'Detach' while agent is mid-execution (isLoading=true), session continues running in background via sessionDetach(), later visible in /resume with running status
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI agent mode
    I want to be prompted with a confirmation modal when exiting AgentView that asks whether to detach (keep session running in background) or close the session
    So that I can start long-running agent tasks, detach to check the kanban board or do other TUI work, and reattach later without losing progress

  Scenario: Detach session using default selection and exit
    Given I am in AgentView with an active session and empty input
    When I press the ESC key
    Then the exit confirmation modal appears with Detach highlighted
    When I press Enter to confirm
    Then the session is detached and continues running in background
    And the view exits


  Scenario: ESC dismisses the exit modal
    Given the exit confirmation modal is showing
    When I press the ESC key
    Then the modal dismisses
    And I remain in AgentView with the session unchanged


  Scenario: No modal when exiting without active session
    Given I am in AgentView with no active session
    When I press the ESC key
    Then the view exits immediately without showing the modal


  Scenario: ESC clears input before showing exit modal
    Given I am in AgentView with an active session and text in the input field
    When I press the ESC key
    Then the input field is cleared
    When I press the ESC key again
    Then the exit confirmation modal appears


  Scenario: Cancel exit and remain in AgentView
    Given I am in AgentView with an active session
    When I press the ESC key
    Then the exit confirmation modal appears
    When I press Right Arrow twice to highlight Cancel
    And I press Enter to confirm
    Then the modal dismisses
    And I remain in AgentView with the session unchanged


  Scenario: Close session and exit
    Given I am in AgentView with an active session and empty input
    When I press the ESC key
    Then the exit confirmation modal appears
    When I press Right Arrow to highlight Close Session
    And I press Enter to confirm
    Then the session is destroyed
    And the view exits

