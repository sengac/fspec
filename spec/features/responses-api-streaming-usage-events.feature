@PROV-038
Feature: Codex provider token tracking shows 0 input tokens — Responses API streaming never emits Usage events
  """
  Fix is in rig-core patched Responses API streaming (codelet/patches/rig-core/src/providers/openai/responses_api/streaming.rs). When response.completed contains usage data, yield RawStreamingChoice::Usage before FinalResponse. Mirrors what Chat Completions API and Anthropic providers already do.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Responses API streaming must yield RawStreamingChoice::Usage before FinalResponse when response.completed contains usage data
  #   2. Usage event must include input_tokens, output_tokens, total_tokens, cache_read_input_tokens, and reasoning_tokens from the ResponsesUsage struct
  #   3. The Usage event must be yielded BEFORE the FinalResponse to ensure stream_loop processes it in the correct order
  #   4. When response.completed has no usage (None), no Usage event should be emitted
  #
  # EXAMPLES:
  #   1. Codex response.completed has usage {input:1000, output:500, total:1500, cached:200, reasoning:300} → stream yields Usage(input=1000, output=500, total=1500, cache_read=200, reasoning=300) then FinalResponse
  #   2. Codex response.completed has no usage field → stream yields only FinalResponse, no Usage event
  #   3. Codex response.completed has usage {input:5000, output:2000, total:7000} with no input_tokens_details → Usage has cache_read=None, reasoning=0
  #   4. Stream yields text deltas followed by Usage then FinalResponse — stream_loop processes Usage event and updates streaming_display before FinalResponse arrives
  #
  # ========================================
  Background: User Story
    As a developer using Codex provider
    I want to see accurate input/output token counts during streaming
    So that I can monitor context window fill and avoid premature compaction

  Scenario: Streaming with usage in response.completed emits Usage event
    Given a Responses API streaming session receives text deltas
    When the streaming response is processed
    Then a RawStreamingChoice::Usage event should be yielded with input_tokens 1000
    And the response.completed event contains usage with input_tokens 1000, output_tokens 500, total_tokens 1500, cached_tokens 200, and reasoning_tokens 300
    And the Usage event should have output_tokens 500 and total_tokens 1500
    And the Usage event should have cache_read_input_tokens 200 and reasoning_tokens 300
    And the Usage event should be yielded before the FinalResponse

  Scenario: Streaming without usage in response.completed skips Usage event
    Given a Responses API streaming session receives text deltas
    When the streaming response is processed
    Then no RawStreamingChoice::Usage event should be yielded
    And the response.completed event contains no usage data
    And only the FinalResponse should be yielded with zero usage values

  Scenario: Streaming with usage but no input_tokens_details handles missing optional fields
    Given a Responses API streaming session receives text deltas
    When the streaming response is processed
    Then a RawStreamingChoice::Usage event should be yielded with input_tokens 5000
    And the response.completed event contains usage with input_tokens 5000, output_tokens 2000, total_tokens 7000 but no input_tokens_details
    And the Usage event should have cache_read_input_tokens None and reasoning_tokens 0
