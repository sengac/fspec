@done
@caching
@providers
@high
@PROV-001
Feature: Optimize Prompt Caching for Multi-Turn Conversations
  """
  This feature modifies transform_user_message_cache_control in codelet/providers/src/caching_client.rs to apply cache_control to the second-to-last user message instead of the first. This enables Anthropic to cache the entire conversation prefix (system + all previous turns) rather than just the system prompt and first user message. The change follows Anthropic's prompt caching documentation which recommends placing cache breakpoints at the end of the cacheable prefix.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic caches content as a prefix from the beginning of the request up to and including the cache_control breakpoint
  #   2. The cache_control breakpoint should be placed on the second-to-last user message to cache the entire conversation prefix
  #   3. For single-turn conversations (only one user message), cache_control should be on that first user message
  #   4. System prompt should always have cache_control applied (already working correctly)
  #
  # EXAMPLES:
  #   1. Single turn: system(cached) + user1(cache_control) - first user message gets cache_control
  #   2. Two turns: system(cached) + user1(cache_control) + assistant1 + user2 - first user message gets cache_control (second-to-last)
  #   3. Three turns: system(cached) + user1 + assistant1 + user2(cache_control) + assistant2 + user3 - second user message gets cache_control
  #   4. Multi-turn with tool use: system(cached) + user1 + assistant1(tool_use) + user2(tool_result) + assistant2 + user3(cache_control) + assistant3 + user4 - third user message gets cache_control
  #   5. Session token usage shows cache_read_tokens increasing with each turn as more conversation history is cached
  #
  # ========================================
  Background: User Story
    As a developer using fspec for multi-turn conversations
    I want to have my conversation history cached efficiently
    So that I reduce API costs and latency in long conversations

  Scenario: Single turn conversation caches first user message
    Given a request body with system prompt and one user message
    When transform_user_message_cache_control is applied
    Then the first user message should have cache_control applied
    And the content should be transformed to array format with cache_control metadata

  Scenario: Two turn conversation caches first user message as second-to-last
    Given a request body with system prompt, user1, assistant1, and user2
    When transform_user_message_cache_control is applied
    Then the first user message should have cache_control applied
    And the second user message should not have cache_control applied

  Scenario: Three turn conversation caches second user message
    Given a request body with system prompt, user1, assistant1, user2, assistant2, and user3
    When transform_user_message_cache_control is applied
    Then the second user message should have cache_control applied
    And the first user message should not have cache_control applied
    And the third user message should not have cache_control applied

  Scenario: Multi-turn conversation with tool use caches correct user message
    Given a request body with system prompt, user1, assistant1 with tool_use, user2 with tool_result, assistant2, user3, assistant3, and user4
    When transform_user_message_cache_control is applied
    Then the third user message should have cache_control applied
    And all other user messages should not have cache_control applied

  Scenario: User message content is transformed to array format
    Given a user message with string content "Hello"
    When cache_control is applied to that message
    Then the content should be an array with one object
    And the object should have type "text"
    And the object should have text "Hello"
    And the object should have cache_control with type "ephemeral"
