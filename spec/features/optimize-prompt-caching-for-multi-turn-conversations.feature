@caching
@providers
@high
@PROV-001
Feature: Optimize Prompt Caching for Multi-Turn Conversations
  """
  This feature modifies transform_user_message_cache_control in codelet/providers/src/caching_client.rs to apply cache_control to the FINAL message instead of just the first user message. Per Anthropic's documentation: "During each turn, we mark the final block of the final message with cache_control so the conversation can be incrementally cached." This enables Anthropic to cache the entire conversation prefix on each turn.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic caches content as a prefix from the beginning of the request up to and including the cache_control breakpoint
  #   2. The cache_control breakpoint should be placed on the FINAL message to enable incremental caching
  #   3. For string content, convert to array format with cache_control on the text block
  #   4. For array content, add cache_control to the last block of the array
  #   5. System prompt should always have cache_control applied (already working correctly)
  #
  # EXAMPLES:
  #   1. Single turn: system(cached) + user1(cache_control) - final message gets cache_control
  #   2. Two turns: system(cached) + user1 + assistant1 + user2(cache_control) - final message gets cache_control
  #   3. Three turns: system(cached) + user1 + assistant1 + user2 + assistant2 + user3(cache_control) - final message gets cache_control
  #   4. Multi-turn with tool use: final message gets cache_control regardless of message type
  #   5. Session token usage shows cache_read_tokens increasing with each turn as more conversation history is cached
  #
  # ========================================
  Background: User Story
    As a developer using fspec for multi-turn conversations
    I want to have my conversation history cached efficiently
    So that I reduce API costs and latency in long conversations

  Scenario: Single turn conversation caches final message
    Given a request body with system prompt and one user message
    When transform_user_message_cache_control is applied
    Then the final message should have cache_control applied
    And the content should be transformed to array format with cache_control metadata

  Scenario: Two turn conversation caches final message
    Given a request body with system prompt, user1, assistant1, and user2
    When transform_user_message_cache_control is applied
    Then the final message (user2) should have cache_control applied
    And the first user message should not have cache_control applied

  Scenario: Three turn conversation caches final message
    Given a request body with system prompt, user1, assistant1, user2, assistant2, and user3
    When transform_user_message_cache_control is applied
    Then the final message (user3) should have cache_control applied
    And no other messages should have cache_control applied

  Scenario: Multi-turn conversation with tool use caches final message
    Given a request body with system prompt, user1, assistant1 with tool_use, user2 with tool_result, assistant2, user3, assistant3, and user4
    When transform_user_message_cache_control is applied
    Then the final message (user4) should have cache_control applied
    And all other messages should not have cache_control applied

  Scenario: Message with string content is transformed to array format
    Given a message with string content "Hello"
    When cache_control is applied to that message
    Then the content should be an array with one object
    And the object should have type "text"
    And the object should have text "Hello"
    And the object should have cache_control with type "ephemeral"

  Scenario: Message with array content has cache_control added to last block
    Given a message with array content containing multiple blocks
    When cache_control is applied to that message
    Then the last block of the array should have cache_control added
    And other blocks should not have cache_control added
