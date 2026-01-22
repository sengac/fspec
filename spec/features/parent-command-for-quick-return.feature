@tui
@watcher-management
@done
@WATCH-014
Feature: /parent Command for Quick Return

  """
  Pattern: Follow existing /watcher command and handleWatcherSelect() session switching pattern
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /parent command is only valid when current session is a watcher (sessionGetParent returns non-null)
  #   2. When valid, /parent switches to the parent session by setting currentSessionId to the parent ID
  #   3. Session switching includes: detaching from current watcher, attaching to parent, restoring parent conversation display
  #   4. When in a regular session (not a watcher), /parent shows an error status message explaining there is no parent
  #   5. When no active session exists, /parent shows an error status message
  #   6. After switching, a status message confirms the switch showing the parent session name or ID
  #
  # EXAMPLES:
  #   1. User is in watcher 'Security Reviewer' observing parent 'Main Dev Session', types /parent → switches to 'Main Dev Session', shows status 'Switched to parent session: Main Dev Session'
  #   2. User is in a regular session 'Code Project' (not a watcher), types /parent → shows error status 'This session has no parent. /parent only works from watcher sessions.'
  #   3. User has no active session (just opened AgentView), types /parent → shows error status 'No active session. Start a session first.'
  #   4. User in watcher types /parent → watcher session is detached (sessionDetach), parent session is attached (sessionAttach), parent conversation is restored via sessionGetMergedOutput
  #   5. User in watcher presses /parent and the parent session was previously streaming → user sees parent conversation at current point, can continue interacting with parent
  #
  # ========================================

  Background: User Story
    As a user in a watcher session
    I want to type /parent to quickly switch back to the parent session
    So that I can efficiently navigate between the watcher and parent without using the watcher overlay

  Scenario: Switch to parent session from watcher
    Given a parent session named "Main Dev Session" exists
    And a watcher session named "Security Reviewer" is attached to "Main Dev Session"
    And the current session is "Security Reviewer"
    When the user types "/parent"
    Then the current session switches to "Main Dev Session"
    And a status message shows "Switched to parent session: Main Dev Session"
    And the parent session conversation is displayed


  Scenario: Error when using /parent in regular session
    Given a regular session named "Code Project" exists
    And the session is not a watcher session
    And the current session is "Code Project"
    When the user types "/parent"
    Then a status message shows "This session has no parent. /parent only works from watcher sessions."
    And the current session remains "Code Project"


  Scenario: Error when no active session exists
    Given no session is currently active
    When the user types "/parent"
    Then a status message shows "No active session. Start a session first."

