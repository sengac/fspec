@done
@tui
@session-management
@persistence
@NAPI-003
Feature: Resume command for session restoration
  """
  Implements resume mode in AgentModal.tsx using React state (isResumeMode, availableSessions, resumeSessionIndex). Uses existing NAPI persistence functions: persistenceListSessions() for fetching, persistenceGetSessionMessages() for restoration. UI follows the /search overlay pattern with full-screen Box, VirtualList-style navigation, and keyboard handling via useInput hook. Sessions sorted client-side by updatedAt descending.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Sessions MUST be filtered to show only sessions belonging to the current project directory
  #   2. Sessions MUST be sorted by updatedAt timestamp in descending order (most recently modified first)
  #   3. Each session entry MUST display: session name, message count, provider name, and relative time since last update
  #   4. The /resume command MUST open a full-screen overlay with a blue double-line border (similar to /search which uses magenta)
  #   5. Arrow Up/Down keys MUST navigate through the session list with visual highlighting of the selected item
  #   6. Enter key MUST select the highlighted session and restore its conversation
  #   7. Escape key MUST cancel the resume mode and return to normal input without making any changes
  #   8. When a session is selected, all messages from that session MUST be loaded and displayed in the conversation view
  #   9. When a session is selected, the currentSessionId state MUST be updated to the selected session's ID for future message persistence
  #   10. After successful restoration, a confirmation message MUST be shown indicating the session name and message count
  #   11. The session list MUST display a maximum of 15 sessions to fit within typical terminal heights
  #   12. When no sessions exist for the current project, a helpful message MUST be displayed instead of an empty list
  #   13. Restoring a session MUST replace the current conversation view entirely (not append to it)
  #   14. Relative time display MUST use human-readable format: 'just now', 'Xm ago', 'Xh ago', 'Xd ago', or date for older sessions
  #   15. The header MUST show the total count of available sessions (e.g., 'Resume Session (5 available)')
  #
  # EXAMPLES:
  #   1. User types /resume, sees overlay with 'Resume Session (3 available)' header, showing: 'Feature work' (12 messages, claude, 5m ago), 'Bug investigation' (8 messages, claude, 2h ago), 'Code review' (23 messages, gemini, 1d ago)
  #   2. User presses Arrow Down twice from first item, highlight moves from 'Feature work' to 'Bug investigation' to 'Code review', wrapping is disabled (cannot go past last item)
  #   3. User presses Arrow Up from 'Code review', highlight moves to 'Bug investigation', pressing Up again moves to 'Feature work', pressing Up again stays at 'Feature work' (no wrap)
  #   4. User highlights 'Bug investigation' and presses Enter, overlay closes, conversation view shows 8 messages from that session, tool message appears: 'Session resumed: "Bug investigation" (8 messages)'
  #   5. User types /resume in a project with no previous sessions, sees overlay with 'Resume Session (0 available)' header and message 'No sessions found for this project'
  #   6. User presses Escape while in resume mode, overlay closes immediately, returns to normal input mode, no changes made to conversation
  #   7. Session updated 30 seconds ago displays as 'just now', session updated 45 minutes ago displays as '45m ago', session updated 3 hours ago displays as '3h ago', session updated 2 days ago displays as '2d ago', session updated 10 days ago displays as '12/13/2025'
  #   8. User has 20 sessions for current project, only the 15 most recent are displayed in the list, older sessions are not shown
  #   9. User has sessions from /Users/dev/projectA and /Users/dev/projectB, when in projectA directory, only projectA sessions appear in the resume list
  #   10. User has current conversation with 5 messages, selects a session with 10 messages, conversation view now shows only the 10 restored messages (previous 5 are replaced, not appended)
  #   11. Session entry displays on two lines: first line shows '> Feature work' (with selection indicator), second line indented shows '12 messages | claude | 5m ago'
  #   12. Footer of overlay displays keyboard hints: 'Enter Select | Arrow Navigate | Esc Cancel'
  #   13. User restores session, sends new prompt, new message is saved to the restored session (not a new session), session's updatedAt timestamp updates
  #   14. Session with no provider set displays 'unknown' in the provider field, e.g., '8 messages | unknown | 2h ago'
  #
  # ========================================
  Background: User Story
    As a developer using the AI agent modal
    I want to see a list of previous sessions and select one to restore
    So that I can continue work from where I left off in a previous conversation

  Scenario: Display session list when resume command is entered
    Given I have 3 sessions saved for the current project
    When I type /resume in the input field
    Then I should see a full-screen overlay with header "Resume Session (3 available)"

  Scenario: Navigate down through session list with arrow keys
    Given I am in resume mode with the first session highlighted
    When I press Arrow Down twice
    Then the third session should be highlighted

  Scenario: Navigate up through session list with arrow keys
    Given I am in resume mode with the last session highlighted
    When I press Arrow Up
    Then the previous session should be highlighted

  Scenario: Select and restore a session with Enter key
    Given I am in resume mode with a session highlighted that has 8 messages
    When I press Enter
    Then the overlay should close and the conversation should show 8 messages with a confirmation

  Scenario: Display empty state when no sessions exist
    Given I have no sessions saved for the current project
    When I type /resume in the input field
    Then I should see the message "No sessions found for this project"

  Scenario: Cancel resume mode with Escape key
    Given I am in resume mode viewing the session list
    When I press Escape
    Then the overlay should close and the conversation should remain unchanged

  Scenario: Display relative time in human-readable format
    Given I have sessions with various update times
    When I type /resume in the input field
    Then sessions should show time as "just now", "Xm ago", "Xh ago", "Xd ago", or date format

  Scenario: Limit displayed sessions to maximum of 15
    Given I have 20 sessions saved for the current project
    When I type /resume in the input field
    Then only the 15 most recent sessions should be displayed

  Scenario: Filter sessions by current project directory
    Given I have sessions from multiple project directories
    When I type /resume in the input field
    Then only sessions from the current project directory should be displayed

  Scenario: Replace current conversation when restoring session
    Given I have a current conversation with 5 messages
    When I restore a session with 10 messages
    Then the conversation should show only the 10 restored messages

  Scenario: Display session entry with proper two-line format
    Given I have sessions saved for the current project
    When I type /resume in the input field
    Then each entry should show the session name on line one and details on line two

  Scenario: Continue conversation in restored session
    Given I have restored a previous session
    When I send a new prompt
    Then the new message should be saved to the restored session

  Scenario: Display unknown for sessions without provider
    Given I have a session saved without a provider specified
    When I type /resume in the input field
    Then the session should display "unknown" as the provider
