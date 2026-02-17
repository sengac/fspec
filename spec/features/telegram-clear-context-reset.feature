@AGENT-022
Feature: Clear context command for session reset
  """
  After clearing session, inject_context_reminders() must be called to restore CLAUDE.md and environment system reminders
  Fix location: codelet/napi/src/session_manager.rs line 4508-4516, the 'clear' action handler
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /clear from Telegram must reset the actual session context (messages, turns, tokens), not just the output buffer
  #   2. After clearing, system reminders (CLAUDE.md, environment info) must be re-injected so the AI retains project context
  #   3. Provider selection, debug mode state, and command history must be preserved across /clear
  #   4. /clear should execute immediately without confirmation prompt
  #
  # EXAMPLES:
  #   1. Remote user sends /clear from Telegram, AI context window is cleared, next message AI doesn't remember previous conversation
  #   2. After /clear, AI still has access to CLAUDE.md project context and environment info (platform, working directory)
  #   3. After /clear, token counters show 0↓ 0↑ (input and output tokens reset)
  #   4. Current implementation clears only output buffer but not session.messages, session.turns, or token_tracker
  #
  # ========================================
  Background: User Story
    As a remote user on Telegram
    I want to send /clear to reset the AI session
    So that start fresh without the AI remembering previous conversation context

  @telegram
  @bridge
  @session
  Scenario: Clear command resets AI context completely
    Given I have an active conversation with the AI via Telegram bridge
    And the conversation has accumulated messages and tokens
    When I send "/clear" via Telegram
    Then the AI should not remember the previous conversation
    And the next message should be treated as a fresh conversation start
    And no confirmation dialog should appear

  @telegram
  @bridge
  @session
  Scenario: System reminders preserved after clear
    Given I have an active conversation with the AI via Telegram bridge
    And the AI has access to project context (CLAUDE.md, environment info)
    When I send "/clear" via Telegram
    Then the AI should still have access to CLAUDE.md project context
    And the AI should still know the platform and working directory
    And the conversation history should be cleared

  @telegram
  @bridge
  @session
  Scenario: Token counters reset after clear
    Given I have an active conversation with accumulated tokens
    And the session shows input and output token counts
    When I send "/clear" via Telegram
    Then the token counters should show "0↓ 0↑"
    And the session.messages should be empty
    And the session.turns should be empty

  @telegram
  @bridge
  @session
  Scenario: Clear resets session state not just output buffer
    Given I have an active conversation via Telegram bridge
    And the session has messages, turns, and token_tracker state
    When I send "/clear" via Telegram
    Then session.messages should be cleared
    And session.turns should be cleared
    And token_tracker should be reset to default
    And inject_context_reminders() should be called to restore system context
