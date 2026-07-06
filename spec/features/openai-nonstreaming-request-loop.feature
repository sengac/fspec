@provider-settings
@streaming
@done
@PROV-140
@rust
@providers
Feature: OpenAI non-streaming request and multi-turn loop

  """
  When streaming is disabled, the OpenAI chat completion request body sets
  stream=false and omits stream_options; when enabled it sets stream=true with
  stream_options.include_usage (unchanged). The non-streaming path is adapted
  into the SAME MultiTurnStreamItem stream the streaming path yields, so the
  interactive driver (run_agent_stream_internal) reuses its existing
  match stream.next() loop and emits the identical terminal StreamEvent
  sequence (Text... then Done). Tool calls in the non-streaming response still
  drive the multi-turn tool loop to completion. Implementation strategy
  (transport-level stream flag vs rig .prompt().multi_turn()) is resolved by a
  time-boxed spike; the observable behaviour in these scenarios is
  strategy-independent.
  """

  Background: User Story
    As a fspec user who disabled streaming on an OpenAI profile
    I want replies and tool calls to work without SSE streaming
    So that endpoints that break on SSE still function end to end

  Scenario: Streaming-disabled request omits streaming options
    Given an OpenAI completion request built with streaming disabled
    When the request body is serialized
    Then the body sets stream to false
    And the body omits stream_options

  Scenario: Streaming-enabled request keeps streaming options
    Given an OpenAI completion request built with streaming enabled
    When the request body is serialized
    Then the body sets stream to true
    And the body includes stream_options with include_usage

  Scenario: Non-streaming text reply is adapted into a Text then final item sequence
    Given a non-streaming OpenAI response containing only assistant text
    When the non-streaming path adapts the response into stream items
    Then a text item carries the assistant text
    And a final response item terminates the sequence

  Scenario: Non-streaming tool call drives the multi-turn loop to completion
    Given a non-streaming OpenAI response requesting a tool call
    When the non-streaming path runs the multi-turn loop
    Then the requested tool is executed
    And the loop continues to a final response
