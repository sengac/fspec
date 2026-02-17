@state-management
@tui
@TUI-066
Feature: TUI /clear should update React state as side effect of Rust state change
  """
  1. Add SessionState::Cleared variant to Rust enum (types.rs)
  2. session.clear_history() emits StreamChunk::SessionStateChange { state: Cleared } after clearing
  3. handleStreamChunk in AgentView.tsx handles 'Cleared' state by resetting conversation/tokenUsage/contextFillPercentage
  4. TUI /clear handler ONLY calls sessionClearHistory() - no manual React state updates
  5. Pattern matches existing SessionStateChange handling (e.g., Compacting)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust session.clear_history() must emit StreamChunk::SessionStateChange { state: Cleared } after clearing messages/turns/tokens
  #   2. TUI handleStreamChunk must handle SessionStateChange with state 'Cleared' by calling setConversation([]), setTokenUsage({inputTokens:0,outputTokens:0}), setContextFillPercentage(0)
  #   3. TUI /clear handler must NOT manually update React state - only call sessionClearHistory and let chunk handler do the rest
  #   4. Bridge /clear and TUI /clear must use the same Rust code path (session.clear_history) with no divergence
  #
  # EXAMPLES:
  #   1. User types /clear in TUI → sessionClearHistory() called → Rust clears state → emits SessionStateChange{Cleared} chunk → handleStreamChunk receives it → resets conversation/tokens/fill% → user sees empty chat
  #   2. Telegram /clear → Bridge control message → Rust clear_history() → emits SessionStateChange{Cleared} → same chunk type as TUI (unified code path)
  #   3. After /clear via TUI, system reminders (CLAUDE.md, environment) are preserved because Rust reinjects them via inject_context_reminders()
  #   4. If sessionClearHistory() throws error, no SessionStateChange{Cleared} chunk emitted, React state unchanged - atomic: either all state updates or none
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have React state update as a side effect of Rust state changes
    So that there is a single source of truth and no state desync between Rust and React

  @integration
  Scenario: TUI /clear triggers Rust state change which emits chunk to update React
    Given I have an active TUI session with conversation history
    And the token counter shows 5000 input tokens
    And the context fill shows 45%
    When I type "/clear" and press Enter
    Then Rust session.clear_history() should be called
    And Rust should emit a SessionStateChange chunk with state "Cleared"
    And the TUI stream handler should receive the chunk
    And the conversation should be reset to empty
    And the token counter should show 0 input tokens
    And the context fill should show 0%

  @integration
  Scenario: Bridge /clear uses same Rust code path as TUI
    Given I have an active Telegram bridge session
    When the Telegram user sends "/clear"
    Then the Bridge should send a control message with action "clear" to Rust
    And Rust session.clear_history() should be called
    And Rust should emit a SessionStateChange chunk with state "Cleared"
    And the chunk type should be identical to TUI /clear flow

  Scenario: System reminders preserved after clear
    Given I have an active TUI session with CLAUDE.md loaded
    And environment info shows project directory and date
    When I type "/clear" and press Enter
    And Rust clears the conversation history
    Then Rust should call inject_context_reminders() after clearing
    And the AI should still know the project context from CLAUDE.md
    And the AI should still know the current date

  Scenario: Clear failure does not corrupt React state
    Given I have an active TUI session with conversation history
    And the token counter shows 5000 input tokens
    When I type "/clear" and press Enter
    And sessionClearHistory fails with an error
    Then no SessionStateChange chunk should be emitted
    And the conversation should remain unchanged
    And the token counter should still show 5000 input tokens
    And there should be no partial state corruption
