@CLI-010
@context-management
@agent
@session-management
@high
Feature: Fix context compaction integration failures

  """
  Architecture notes:
  Key architectural decisions: Session owns token_tracker and turns fields. Messages are converted to ConversationTurn after each agent response completes. Compaction uses anchor-based strategy with TurnSelector and AnchorDetector from compaction module. Dependencies: rig's streaming API for token usage, Session struct from session module, compaction types from agent::compaction. Critical requirements: Must capture MessageStart.usage and FinalResponse for complete token breakdown. Must check effective_tokens() > 90% threshold after each turn. Must reconstruct messages array with summary + kept turns after compaction.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Token usage MUST be extracted from rig streaming responses after each agent turn
  #   2. Session.token_tracker MUST accumulate tokens across all turns in the conversation
  #   3. Messages MUST be converted to ConversationTurn after each agent response completes
  #   4. Compaction MUST trigger when effective_tokens() exceeds 90% of context window (accounting for AUTOCOMPACT_BUFFER)
  #   5. When compaction triggers, system MUST detect anchors, select turns, generate summary, and reconstruct messages array
  #   6. After compaction, Session.messages MUST contain: system messages + summary + continuation + kept turn messages
  #
  # EXAMPLES:
  #   1. User has 10 turn conversation totaling 95k tokens → after turn 10, token_tracker shows 95k input tokens → compaction triggers → messages compacted to 40k tokens → conversation continues
  #   2. Agent response includes tool call (Edit) + tool result (test pass) → messages converted to ConversationTurn with tool_calls=['Edit'] and tool_results=[success=true] → turn added to Session.turns
  #   3. Session at 92k effective tokens → next turn adds 5k → reaches 97k > 90k threshold → compaction runs automatically before returning control to user
  #   4. Compaction detects anchor at turn 7 (error resolution) → keeps turns 7-10, summarizes turns 0-6 → messages array reconstructed with summary message + kept turn messages
  #   5. After compaction, user sends new message → agent has access to summary + recent turns → conversation context preserved correctly
  #
  # QUESTIONS (ANSWERED):
  #   Q: How do we extract token usage from rig's streaming responses? Does it provide usage in the stream or only after completion?
  #   A: Extract from rig streaming: MessageStart event contains Usage struct with cache_read_input_tokens and cache_creation_input_tokens, FinalResponse contains output_tokens. Must capture MessageStart.usage and accumulate into Session.token_tracker after each agent turn completes.
  #
  #   Q: @ai-research: What is the exact structure of rig's streaming response that contains token usage for Anthropic provider?
  #   A: Rig provides token usage through MessageStart event (has input_tokens, cache_read_input_tokens, cache_creation_input_tokens) and FinalResponse at stream end (has output_tokens and cumulative input_tokens). We need to capture both to get complete usage with cache breakdown.
  #
  # ========================================

  Background: User Story
    As a developer using codelet for long coding sessions
    I want to have context automatically compact when approaching limits
    So that I can continue working without manual session resets or losing conversation history

  Scenario: Extract token usage from rig streaming in interactive.rs
    Given I start an interactive REPL session
    And I send a message to the agent
    When the agent streams a response with token usage metadata
    Then Session.token_tracker should be updated with input_tokens from MessageStart
    And Session.token_tracker should be updated with output_tokens from FinalResponse
    And Session.token_tracker should be updated with cache_read_input_tokens

  Scenario: Accumulate tokens across multiple turns in interactive.rs
    Given I am in an interactive REPL session
    And I have had 3 conversation turns
    When each turn adds 1000 input tokens and 100 output tokens
    Then Session.token_tracker.input_tokens should be 3000
    And Session.token_tracker.output_tokens should be 300
    And token values persist in Session across REPL iterations

  Scenario: Convert messages to ConversationTurn in interactive.rs
    Given I am in an interactive REPL session
    And an agent response includes tool calls and results
    When the agent response completes
    Then a ConversationTurn should be created from the messages
    And the turn should be added to Session.turns
    And the turn should include tool_calls and tool_results

  Scenario: Check compaction threshold in interactive.rs
    Given Session is at 92000 effective tokens
    And context window is 100000 with 90% threshold at 90000
    When next turn adds 5000 tokens reaching 97000 total
    Then compaction trigger check should execute in interactive.rs
    And compaction should be triggered because 97000 > 90000

  Scenario: Execute compaction when triggered in interactive.rs
    Given compaction threshold has been exceeded
    When compaction trigger check detects this
    Then ContextCompactor.compact() should be called
    And anchor detection should run on Session.turns
    And turn selection should determine kept vs summarized turns
    And LLM summary should be generated for old turns

  Scenario: Reconstruct messages after compaction in interactive.rs
    Given Session has 10 turns with turn 7 as error-resolution anchor
    When compaction executes
    Then Session.messages should be cleared
    And system messages should be preserved and added first
    And summary message should be added as User message
    And continuation message should be added
    And kept turn messages (turns 7-9) should be converted and added

  Scenario: Reduce token count after compaction in interactive.rs
    Given Session is at 97000 effective tokens before compaction
    When compaction executes and completes
    Then token count should be recalculated for new messages
    And Session.token_tracker should reflect compacted size
    And effective_tokens() should be below 90000 threshold

  Scenario: Preserve system messages during compaction in interactive.rs
    Given Session has system messages at start of messages array
    And Session has accumulated conversation turns
    When compaction executes
    Then system messages MUST be preserved in reconstructed messages
    And system messages MUST be at the beginning of the array
    And summary comes AFTER system messages

  Scenario: Notify user during compaction in interactive.rs
    Given compaction is about to execute
    When compaction starts
    Then user should see "[Generating summary...]" notification
    When compaction completes
    Then user should see compaction metrics with tokens and compression percentage

  Scenario: Full end-to-end compaction integration flow
    Given I start an interactive REPL session
    When I have multiple turns that exceed the compaction threshold
    Then token extraction should happen after each turn
    And token accumulation should track total usage
    And turn conversion should populate Session.turns
    And compaction check should detect threshold exceeded
    And compaction should execute automatically
    And Session.messages should be reconstructed
    And conversation should continue with compacted context
