@session-management
@tui
@TUI-065
Feature: TUI /clear command only clears React state, not actual session context

  """
  The bug was a simple typo: code used 'currentSessionRef.current' but the variable is 'currentSessionIdRef'. Fixed by using 'currentSessionId' directly which is already in scope.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After /clear, the AI must have ZERO memory of prior conversation - the messages sent to the LLM API must not include any previous turns
  #   2. System reminders (CLAUDE.md, environment info) must be preserved - these are project context, not conversation history
  #
  # EXAMPLES:
  #   1. User discusses topic X for 5 turns, types /clear, asks 'what were we talking about?' - AI should NOT know about topic X
  #   2. After /clear, AI still knows it's working on fspec project (from CLAUDE.md) and the current date (from environment)
  #   3. Token counters reset to 0 after /clear (both displayed UI and actual session state)
  #
  # QUESTIONS (ANSWERED):
  #   Q: WHY is /clear not working? The session_clear_history NAPI exists and is called, inner.messages.clear() runs, but AI still remembers conversation. Is inner.messages NOT what gets sent to API? Is there caching?
  #   A: INVESTIGATION NEEDED: The Rust implementation clears inner.messages but we need to verify the API request is actually being built from inner.messages and not some cached/buffered copy. Also need to verify sessionClearHistory NAPI is actually being called from TypeScript.
  #
  # ========================================

  Background: User Story
    As a developer
    I want to use /clear in the TUI to reset the AI session
    So that start fresh without the AI remembering the previous conversation

  Scenario: AI has no memory of prior conversation after clear
    Given I have a TUI session with an active conversation
    And I have discussed "topic X" with the AI for 5 turns
    When I type "/clear" and press Enter
    And I ask "what were we talking about?"
    Then the AI should NOT know about "topic X"

  Scenario: System reminders preserved after clear
    Given I have a TUI session with CLAUDE.md loaded
    And environment info shows the current date
    When I type "/clear" and press Enter
    And I ask "what project are you working on?"
    Then the AI should still know it's working on fspec project
    And the AI should still know the current date

  Scenario: Token counters reset after clear
    Given I have a TUI session with token usage showing 5000 input tokens
    When I type "/clear" and press Enter
    Then the displayed token counter should show 0
    And the Rust session token tracker should show 0
