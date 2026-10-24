@PROV-038
@providers
@context-management
@phase-1
Feature: Responses API streaming validates and normalizes terminal usage
  """
  The patched rig-core Responses API stream follows the terminal evidence and inclusive normalization contracts in builtin-provider-terminal-usage-evidence.feature and inclusive-provider-usage.feature. Valid terminal usage emits normalized Usage before FinalResponse. Missing terminal usage returns an explicit error without Usage or FinalResponse.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Responses API streaming must yield RawStreamingChoice::Usage before FinalResponse when response.completed contains usage data
  #   2. Inclusive input and output counts must be normalized into disjoint input, cached, output, and reasoning categories while preserving the inclusive total
  #   3. The Usage event must be yielded BEFORE the FinalResponse to ensure stream_loop processes it in the correct order
  #   4. When response.completed has no usage (None), return ResponseError "missing Responses usage" without emitting Usage or FinalResponse
  #
  # EXAMPLES:
  #   1. Codex response.completed has usage {input:1000, output:500, total:1500, cached:200, reasoning:300} → stream yields Usage(input=800, output=200, total=1500, cache_read=200, reasoning=300) then FinalResponse with the same normalized usage
  #   2. Codex response.completed has null usage → stream returns ResponseError "missing Responses usage", no Usage event and no FinalResponse
  #   3. Codex response.completed has usage {input:5000, output:2000, total:7000} with no input_tokens_details → Usage has cache_read=None, reasoning=0
  #   4. Stream yields text deltas followed by Usage then FinalResponse — stream_loop processes Usage event and updates streaming_display before FinalResponse arrives
  #
  # ========================================
  Background: User Story
    As a developer using Codex provider
    I want to see accurate input/output token counts during streaming
    So that I can monitor context window fill and avoid premature compaction

  Scenario: Streaming with inclusive usage emits normalized Usage before FinalResponse
    Given a Responses API streaming session receives text deltas
    And the response.completed event contains no usage data
    And the response.completed event contains usage with input_tokens 1000, output_tokens 500, total_tokens 1500, cached_tokens 200, and reasoning_tokens 300
    When the streaming response is processed
    Then a RawStreamingChoice::Usage event should be yielded with input_tokens 800
    And the Usage event should have output_tokens 200 and total_tokens 1500
    And the Usage event should have cache_read_input_tokens 200 and reasoning_tokens 300
    And the Usage event should be yielded before the FinalResponse
    And exactly one FinalResponse should contain the same normalized usage

  Scenario: Streaming without terminal usage reports an error without terminal success
    Given a Responses API streaming session receives text deltas
    When the streaming response is processed
    Then a ResponseError with message "missing Responses usage" should be returned
    And no RawStreamingChoice::Usage event should be yielded
    And no FinalResponse should be yielded

  Scenario: Streaming with usage but no input_tokens_details handles missing optional fields
    Given a Responses API streaming session receives text deltas
    And the response.completed event contains usage with input_tokens 5000, output_tokens 2000, total_tokens 7000 but no input_tokens_details
    When the streaming response is processed
    Then a RawStreamingChoice::Usage event should be yielded with input_tokens 5000
    And the Usage event should have cache_read_input_tokens None and reasoning_tokens 0
