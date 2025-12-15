@CLI-014
Feature: Extract cache tokens from Anthropic API response

  """
  Implementation approach:

  For STREAMING responses:
  - rig's streaming abstracts away Anthropic-specific cache fields
  - MessageStart.usage HAS cache_read_input_tokens/cache_creation_input_tokens
  - BUT PartialUsage and FinalResponse don't propagate them
  - Workaround: Use a streaming hook to intercept raw SSE and parse MessageStart

  For NON-STREAMING responses:
  - raw_response contains full Anthropic CompletionResponse
  - raw_response.usage.cache_read_input_tokens is accessible

  Implementation:
  - Add extract_anthropic_cache_tokens() helper to parse raw Usage
  - For streaming: Document limitation, cache tokens = None until rig PR
  - For non-streaming: Extract from raw_response.usage
  - TokenTracker infrastructure already in place
  - effective_tokens() discount formula already implemented
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Cache read tokens MUST be extracted from Anthropic API streaming response usage data
  #   2. Cache creation tokens MUST be extracted from Anthropic API streaming response usage data
  #   3. Extracted cache tokens MUST be accumulated into session.token_tracker after each turn
  #   4. Non-Anthropic providers MUST gracefully handle missing cache token fields (default to None/0)
  #   5. For Anthropic provider, cache tokens MUST be extracted by parsing the raw SSE MessageStart event directly, since rig's streaming abstraction loses this information
  #
  # EXAMPLES:
  #   1. Anthropic response contains cache_read_input_tokens=5000, this value is extracted and added to session.token_tracker.cache_read_input_tokens
  #   2. Anthropic response contains cache_creation_input_tokens=2000, this value is extracted and added to session.token_tracker.cache_creation_input_tokens
  #   3. After extracting cache_read=5000, effective_tokens() returns input_tokens - (5000 * 0.9) = input_tokens - 4500
  #   4. OpenAI provider response has no cache fields, cache_read_input_tokens remains None, effective_tokens() returns full input_tokens
  #   5. When Anthropic SSE MessageStart event contains usage.cache_read_input_tokens=5000 and usage.cache_creation_input_tokens=2000, these values are extracted and stored in turn_cache_read_tokens and turn_cache_creation_tokens
  #
  # QUESTIONS (ANSWERED):
  #   Q: @ai: rig 0.25.0 streaming implementation doesn't expose cache tokens - the Anthropic MessageStart event has full Usage with cache_read_input_tokens, but this info is lost when converting to PartialUsage and FinalResponse. Options: 1) Intercept MessageStart event directly before rig aggregates, 2) Fork/patch rig to expose cache tokens, 3) Use non-streaming API (loses streaming UX)
  #   A: Solution: We need to intercept the raw SSE MessageStart event which contains full Anthropic Usage (with cache fields) before rig aggregates it. Two approaches: 1) PREFERRED: Add a custom event handler in our streaming loop that extracts cache tokens from MessageStart before yielding to rig's processing - this requires parsing the raw SSE ourselves or using rig's lower-level streaming API. 2) Contribute PR to rig to expose cache tokens. We will implement option 1 by processing rig's StreamedAssistantContent::Final which contains the provider-specific StreamingCompletionResponse - but this only has PartialUsage without cache fields. So we must parse SSE directly for MessageStart. Will need to wrap rig's agent or use provider-specific streaming.
  #
  # ========================================

  Background: User Story
    As a AI agent system
    I want to accurately track cache token usage from Anthropic API responses
    So that the effective_tokens() calculation correctly applies the 90% cache discount, enabling proper compaction threshold decisions

  Scenario: Extract cache read tokens from Anthropic SSE MessageStart
    Given an Anthropic SSE MessageStart event with usage containing cache_read_input_tokens=5000
    When the streaming response is processed
    Then turn_cache_read_tokens should be set to 5000


  Scenario: Extract cache creation tokens from Anthropic SSE MessageStart
    Given an Anthropic SSE MessageStart event with usage containing cache_creation_input_tokens=2000
    When the streaming response is processed
    Then turn_cache_creation_tokens should be set to 2000


  Scenario: Effective tokens calculation applies 90% cache discount
    Given a TokenTracker with input_tokens=10000 and cache_read_input_tokens=5000
    When effective_tokens() is called
    Then the result should be 5500 (10000 - 5000 * 0.9)


  Scenario: Non-Anthropic providers default to no cache tokens
    Given an OpenAI provider streaming response without cache fields
    When the streaming response is processed
    Then turn_cache_read_tokens should remain None
    And turn_cache_creation_tokens should remain None
    And effective_tokens() should return the full input_tokens value


  Scenario: Accumulate cache tokens into session tracker after turn
    Given a session with token_tracker.cache_read_input_tokens=1000
    And turn_cache_read_tokens=5000 extracted from a completed turn
    When the turn completes and tokens are accumulated
    Then session.token_tracker.cache_read_input_tokens should be 6000

