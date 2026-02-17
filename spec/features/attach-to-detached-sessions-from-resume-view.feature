@session-management
@tui
@done
@TUI-047
Feature: Attach to Detached Sessions from Resume View

  """
  Modifies handleResumeMode() to query both persistenceListSessions() and sessionManagerList(), merging results with background sessions taking precedence. Adds status icon rendering (🔄/💾) in resume list. Uses GlobalSessionStreamManager to register handlers for streaming sessions.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Resume view must query both persistenceListSessions() AND sessionManagerList() to get full picture of sessions
  #   2. Each session in resume list must display status icon: 🔄 for 'running' background sessions, 💾 for 'idle' persisted sessions
  #   3. Background sessions from sessionManagerList() take precedence - if session exists in both lists, use background session data and show running/idle status emoji
  #   4. When user selects a RUNNING background session (🔄), use GlobalSessionStreamManager handler registration instead of persistenceLoadSession()
  #   5. When attaching to running session, first call sessionGetBufferedOutput(sessionId, limit) to hydrate conversation with output produced while detached
  #   6. After buffer hydration, GlobalSessionStreamManager handler receives live streaming chunks that append to conversation in real-time
  #   7. When user selects an IDLE persisted session (💾), use existing persistenceLoadSession() code path (no change to current resume behavior)
  #   8. Session list must be sorted by updatedAt descending (most recent first), same as current behavior
  #   9. Footer in resume mode must show: 'Enter Select | ↑↓ Navigate | D Delete | Esc Cancel' (unchanged)
  #   10. D key delete functionality (TUI-040) must work for both running and persisted sessions
  #
  # EXAMPLES:
  #   1. User types /resume, sees list of sessions: '🔄 Long running task' (running), '💾 Yesterday session' (idle), user selects running session, immediately sees live output streaming
  #   2. User detaches from session (TUI-046), goes to kanban board, presses 'A' to open agent mode, types /resume, sees their session with 🔄 icon, selects it, sees buffered output then live stream
  #   3. User has 3 sessions: 2 running in background (🔄🔄) and 1 from yesterday (💾), all appear in resume list sorted by most recent first
  #   4. User selects idle session (💾), session loads from persistence using existing code path, conversation history displays as before
  #   5. User presses D on running session (🔄), delete dialog appears, user confirms, session is destroyed via sessionManagerDestroy(), list refreshes
  #   6. User attaches to running session, agent produces tool call while watching, user sees '● Bash(command)' appear in real-time, then sees tool result stream in
  #   7. User attaches to session that finished while detached, buffered output shows complete response, session status shows idle, user can type new prompt
  #   8. User has no background sessions but has persisted sessions, resume view shows only 💾 icons, no 🔄 icons, all sessions load from persistence (backward compatible)
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to see which sessions are running in the background when viewing /resume, indicated by emoji icons, and attach to them using the existing background session API rather than loading from persistence
    So that I can quickly reattach to a running session and immediately see live streaming output, getting instant visibility into agent progress without waiting for data to load from disk

  Scenario: Resume view shows running and idle sessions with status icons
    Given I have a running background session and an idle persisted session
    When I type /resume in AgentView
    Then I see the running session with a 🔄 icon
    And I see the idle session with a 💾 icon


  Scenario: Attach to running session shows buffered output then live stream
    Given I have a detached session that produced output while I was away
    When I select the running session from /resume
    Then I immediately see the buffered output from while I was detached
    And I see live streaming output continue in real-time


  Scenario: Idle session loads from persistence as before
    Given I have an idle persisted session with conversation history
    When I select the idle session from /resume
    Then the conversation history displays as before
    And I can type a new prompt


  Scenario: Delete running session destroys background session
    Given I am in resume mode with a running background session selected
    When I press D and confirm deletion
    Then the background session is destroyed
    And the session list refreshes without that session


  Scenario: Sessions sorted by most recent first
    Given I have multiple sessions with different last updated times
    When I view the resume list
    Then sessions appear sorted by most recent first


  Scenario: Backward compatible when no background sessions exist
    Given I have only persisted sessions and no background sessions
    When I view the resume list
    Then I see all sessions with 💾 icons
    And selecting any session loads from persistence as before


  Scenario: Attach to idle background session shows complete output
    Given I have a session that finished while I was detached
    When I select the idle background session from /resume
    Then I see the complete buffered output from the finished task
    And I can type a new prompt to continue the conversation

