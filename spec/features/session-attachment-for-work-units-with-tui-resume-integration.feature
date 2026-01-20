@done
@session-management
@tui
@high
@SESS-001
Feature: Session attachment for work units with TUI resume integration

  """
  Architecture notes:

  State Management (fspecStore.ts):
  - Add sessionAttachments: Map<workUnitId, sessionId> to store state
  - Add currentWorkUnitId: string | null to track which work unit user entered
  - Add actions: attachSession(workUnitId, sessionId), detachSession(workUnitId), getAttachedSession(workUnitId)
  - State is in-memory only, cleared when TUI process exits

  BoardView → AgentView Flow:
  - BoardView passes selectedWorkUnitId to AgentView when user presses Enter
  - AgentView receives workUnitId prop and stores in currentWorkUnitId
  - On mount, AgentView checks store.getAttachedSession(workUnitId)
  - If session found: call persistenceGetSessionMessages() to restore conversation
  - If no session: start fresh, attach on first message via persistenceCreateSessionWithProvider()

  Visual Indicator (UnifiedBoardLayout.tsx):
  - Read sessionAttachments from store
  - When rendering work unit ID, check if workUnitId has attached session
  - Prepend "🟢 " to work unit ID text if session attached

  Command Handlers (AgentView.tsx):
  - /detach: Call store.detachSession(currentWorkUnitId), then clear conversation state
  - /resume: After user selects session, call store.attachSession(currentWorkUnitId, selectedSessionId)
  - Space+ESC: Returns to board without detaching (session stays attached and running)

  Integration with existing NAPI:
  - Uses persistenceCreateSessionWithProvider() for new sessions (existing)
  - Uses persistenceGetSessionMessages() for resume (existing)
  - Uses persistenceListSessions() for /resume list (existing)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A work unit can have at most one session attached at a time
  #   2. Session auto-attaches to the current work unit when the first message is sent (session creation)
  #   3. Work units with an attached session display a 🟢 green circle indicator in the board view
  #   4. Pressing Enter on a work unit with an attached session resumes that session instead of starting fresh
  #   5. /detach command detaches the current session from the work unit and starts a fresh empty session
  #   6. /resume attaches the selected session to the currently entered work unit
  #   7. Session attachments persist until the app closes (in-memory state, not persisted to disk)
  #   8. The 🟢 indicator appears on the left side of the work unit ID (e.g., 🟢 AUTH-001)
  #   9. Space+ESC returns to board but keeps session attached to work unit (session continues in background)
  #
  # EXAMPLES:
  #   1. User presses Enter on AUTH-001 (no attached session), types first message -> session is created and auto-attached to AUTH-001
  #   2. User views board, AUTH-001 has attached session -> AUTH-001 displays with 🟢 indicator
  #   3. User presses Enter on AUTH-001 (has attached session) -> conversation resumes with previous messages restored
  #   4. User types /detach while in AUTH-001 session -> session detached from AUTH-001, conversation cleared, fresh session ready
  #   5. User types /resume while in AUTH-001, selects a session -> selected session attaches to AUTH-001, messages restored
  #   6. User views board, AUTH-001 has no attached session -> AUTH-001 displays without 🟢 indicator (normal display)
  #   7. User closes and reopens TUI -> all session attachments are cleared (in-memory only)
  #   8. User /resume on AUTH-001 (already has session-A attached), selects session-B -> session-B replaces session-A as the attachment
  #   9. User presses Space+ESC while in AUTH-001 session -> returns to board, AUTH-001 still shows 🟢 indicator, session continues in background
  #
  # QUESTIONS (ANSWERED):
  #   Q: If you /resume while in a work unit that already has a different session attached, does the new session replace the old one? Or should there be a prompt/warning?
  #   A: New session replaces the old attachment (no prompt/warning)
  #
  #   Q: What happens when you press Space+ESC (session detach)? Does that also detach the session from the work unit, or just return to board while keeping session attached?
  #   A: Space+ESC only returns to board, session stays attached to work unit and continues running in background
  #
  #   Q: Where should the 🟢 indicator appear relative to the work unit ID? Before it (🟢 AUTH-001) or after it (AUTH-001 🟢)?
  #   A: Left side: 🟢 AUTH-001
  #
  # ========================================

  Background: User Story
    As a developer using codelet TUI
    I want to have my conversation session automatically attached to the work unit I'm working on
    So that when I return to that work unit, I can resume exactly where I left off without searching through session history

  @auto-attach
  Scenario: Session auto-attaches to work unit on first message
    Given I am viewing the board with work unit "AUTH-001" selected
    And "AUTH-001" has no attached session
    When I press Enter to open the agent view
    And I send my first message
    Then a new session should be created
    And that session should be automatically attached to "AUTH-001"

  @visual-indicator
  Scenario: Work unit with attached session displays green indicator
    Given work unit "AUTH-001" has an attached session
    When I view the board
    Then "AUTH-001" should display with a 🟢 indicator on the left side
    And the display should show "🟢 AUTH-001"

  @visual-indicator
  Scenario: Work unit without attached session displays normally
    Given work unit "AUTH-001" has no attached session
    When I view the board
    Then "AUTH-001" should display without any session indicator

  @resume-on-enter
  Scenario: Pressing Enter on work unit with attached session resumes conversation
    Given work unit "AUTH-001" has an attached session with previous messages
    When I press Enter on "AUTH-001"
    Then the agent view should open
    And the previous conversation messages should be restored
    And I can continue the conversation from where I left off

  @detach-command
  Scenario: Detach command removes session attachment and clears conversation
    Given I am in the agent view for work unit "AUTH-001"
    And "AUTH-001" has an attached session with messages
    When I type "/detach"
    Then the session should be detached from "AUTH-001"
    And the conversation should be cleared
    And I should have a fresh empty session ready

  @resume-command
  Scenario: Resume command attaches selected session to current work unit
    Given I am in the agent view for work unit "AUTH-001"
    And "AUTH-001" has no attached session
    And there are existing sessions available
    When I type "/resume"
    And I select a session from the list
    Then the selected session should be attached to "AUTH-001"
    And the session messages should be restored

  @resume-command
  Scenario: Resume command replaces existing session attachment
    Given I am in the agent view for work unit "AUTH-001"
    And "AUTH-001" has "session-A" attached
    And there are other sessions available including "session-B"
    When I type "/resume"
    And I select "session-B" from the list
    Then "session-B" should replace "session-A" as the attachment for "AUTH-001"
    And "session-B" messages should be restored

  @space-esc
  Scenario: Space+ESC returns to board while keeping session attached
    Given I am in the agent view for work unit "AUTH-001"
    And "AUTH-001" has an attached session
    When I press Space+ESC
    Then I should return to the board view
    And "AUTH-001" should still show the 🟢 indicator
    And the session should continue running in the background

  @persistence
  Scenario: Session attachments are cleared when app closes
    Given work unit "AUTH-001" has an attached session
    And work unit "UI-002" has an attached session
    When I close and reopen the TUI application
    Then "AUTH-001" should have no attached session
    And "UI-002" should have no attached session
    And no work units should display the 🟢 indicator
