@AGENT-003 @tui @agent @session-management
Feature: Clear context command for session reset

  """
  Architecture notes:
  - /clear must reset session to equivalent of fresh CodeletSession state
  - CRITICAL: clear_history() destroys system reminders (CLAUDE.md, environment info)
  - MUST call inject_context_reminders() after clearing to restore project context
  - Without reinjecting reminders, AI loses CLAUDE.md context on next prompt
  - Implementation pattern: clear messages/turns/tokens → reinject reminders

  Layer responsibilities:
  - Rust/NAPI: session.messages.clear(), session.turns.clear(), token_tracker=0, inject_context_reminders()
  - React: setConversation([]), setTokenUsage({inputTokens:0, outputTokens:0})

  Preserved state (NOT cleared):
  - currentProvider (session.provider_manager unchanged)
  - isDebugEnabled (React state)
  - historyEntries (command history for Shift+↑↓)
  - currentSessionId (persistence session continues)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /clear resets the session to a fresh state - clears displayed conversation, message history, and token counters
  #   2. /clear preserves current provider selection, debug mode state, and command history
  #   3. /clear executes immediately without confirmation prompt
  #
  # EXAMPLES:
  #   1. User has 20-message conversation with 5000 tokens, types /clear, conversation becomes empty and tokens show 0↓ 0↑
  #   2. User switches to OpenAI provider, types /clear, provider remains OpenAI after clear
  #   3. User enables debug mode with /debug, types /clear, [DEBUG] indicator remains visible
  #   4. User has command history from previous prompts, types /clear, Shift+↑ still shows previous commands
  #
  # ========================================

  Background: User Story
    As a developer using the AI agent
    I want to clear the context window and reset the session
    So that I can start a fresh conversation without closing and reopening the modal

  Scenario: Clear conversation and reset tokens to fresh state
    Given I have a conversation with messages and accumulated tokens
    When I type "/clear" and press Enter
    Then the conversation area should show "Type a message to start..."
    And the token counter should show "0↓ 0↑"
    And no confirmation dialog should appear

  Scenario: Preserve provider selection after clear
    Given I have switched to the "openai" provider
    And I have an ongoing conversation
    When I type "/clear" and press Enter
    Then the header should still show "Agent: openai"
    And the conversation area should be empty

  Scenario: Preserve debug mode after clear
    Given I have enabled debug mode with "/debug"
    And I have an ongoing conversation
    When I type "/clear" and press Enter
    Then the "[DEBUG]" indicator should still be visible in the header
    And the conversation area should be empty

  Scenario: Preserve command history after clear
    Given I have sent several prompts in this session
    When I type "/clear" and press Enter
    And I press Shift+Arrow-Up
    Then I should see my previous commands in the input field
