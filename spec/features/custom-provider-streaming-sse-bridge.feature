@done
@streaming
@validator
@rust
@providers
@PROV-064
Feature: Custom provider streaming SSE bridge
  """
  SSE frames parsed in Rust via eventsource-stream; Rhai invoked per-event; StreamChunk is a new enum in custom::stream; tool call deltas accumulated in Rust; runtime errors yield single Err; [DONE] terminates
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SSE frame parsing stays in Rust (reqwest + eventsource-stream); Rhai only interprets each data payload
  #   2. parse_stream_chunk(config, event_data_string) returns a map describing one of: text_delta, tool_call_delta, stop, ignore, or error
  #   3. build_stream_request(config, messages, tools, model) returns the JSON body used for streaming requests (may set stream:true flag)
  #   4. SSE events with data == '[DONE]' are treated as end-of-stream without invoking parse_stream_chunk
  #   5. Tool call deltas are accumulated in Rust: partial JSON input strings are concatenated until a tool call's arguments parse successfully
  #   6. Each parse_stream_chunk runs inside tokio::task::spawn_blocking so the async runtime is never blocked
  #   7. The streaming bridge yields an async Stream<Item = Result<StreamChunk, ProviderError>> where StreamChunk variants cover TextDelta, ToolCallStart, ToolCallArgsDelta, ToolCallComplete, StopReason
  #   8. A parse_stream_chunk runtime error does NOT kill the whole stream — it is surfaced as a single StreamChunk Err and streaming terminates gracefully
  #   9. HTTP error responses (>=400) during streaming requests still flow through map_error and produce ProviderError before any stream item is yielded
  #
  # EXAMPLES:
  #   1. Feeding OpenAI-style SSE {data: {choices:[{delta:{content:'Hel'}}]}} yields a TextDelta('Hel') chunk
  #   2. Two consecutive SSE content deltas ('Hel' then 'lo') produce two TextDelta chunks in order
  #   3. An SSE event with data:'[DONE]' causes the stream to complete without calling parse_stream_chunk
  #   4. A tool_call_delta with partial arguments ('{"pa') followed by ('th":"a.txt"}') accumulates into a single ToolCallComplete with parsed input {path:'a.txt'}
  #   5. An SSE event with finish_reason:'stop' produces a StopReason(EndTurn) chunk
  #   6. An SSE event with finish_reason:'tool_calls' produces a StopReason(ToolUse) chunk
  #   7. parse_stream_chunk returning {kind:'ignore'} for a keepalive event produces NO emitted chunk
  #   8. When parse_stream_chunk throws a runtime error, the stream yields a single Err(ProviderError::Api) and then terminates
  #   9. A streaming HTTP request returning 401 Unauthorized yields a single Err(ProviderError::Auth) before any Ok chunk
  #   10. build_stream_request returns a body with 'stream':true which is sent to the wiremock SSE endpoint
  #   11. End-to-end streaming against a wiremock SSE server returning 3 content deltas + 1 stop event yields 4 stream chunks (3 TextDelta, 1 StopReason) in order
  #
  # ========================================
  Background: User Story
    As a custom provider author
    I want to have my Rhai script incrementally parse SSE events into streaming chunks (text deltas, tool call deltas, stop events)
    So that users see tokens stream in real time from any LLM API without recompiling fspec

  Scenario: Emit TextDelta chunk for single content delta
    Given a Rhai script whose parse_stream_chunk extracts delta.content as text_delta
    When the SSE stream emits data '{"choices":[{"delta":{"content":"Hel"}}]}'
    Then the bridge yields one StreamChunk::TextDelta with value "Hel"

  Scenario: Emit TextDelta chunks in order for consecutive content deltas
    Given a Rhai script extracting text_delta from delta.content
    When the SSE stream emits two content deltas "Hel" then "lo"
    Then the bridge yields TextDelta("Hel") followed by TextDelta("lo")

  Scenario: Terminate stream on DONE marker without invoking parse_stream_chunk
    Given any valid streaming provider configuration
    When the SSE stream emits data "[DONE]"
    Then the bridge completes without yielding further chunks and parse_stream_chunk is not invoked for that event

  Scenario: Accumulate partial tool call arguments into a single ToolCallComplete
    Given a Rhai script emitting tool_call_delta with incremental arguments
    When the stream emits arguments "{\"pa" then "th\":\"a.txt\"}" followed by finish_reason "tool_calls"
    Then the bridge yields one ToolCallComplete whose input equals {"path":"a.txt"}

  Scenario: Emit StopReason EndTurn for stop finish_reason
    Given a Rhai script that maps finish_reason to the stop kind
    When the stream emits finish_reason "stop"
    Then the bridge yields StreamChunk::StopReason(EndTurn)

  Scenario: Emit StopReason ToolUse for tool_calls finish_reason
    Given a Rhai script mapping finish_reason "tool_calls" to tool_use
    When the stream emits finish_reason "tool_calls"
    Then the bridge yields StreamChunk::StopReason(ToolUse)

  Scenario: Skip events when parse_stream_chunk returns ignore
    Given a Rhai script that returns kind "ignore" for keepalive events
    When the SSE stream emits a keepalive event
    Then no StreamChunk is yielded for that event

  Scenario: Yield error and terminate on Rhai runtime error
    Given a Rhai script whose parse_stream_chunk throws a runtime error
    When the SSE stream emits any event the script throws on
    Then the bridge yields a single Err(ProviderError::Api) and then terminates

  Scenario: Yield auth error before any chunk on 401 streaming response
    Given a wiremock server responding with 401 to the streaming endpoint
    When RhaiCustomProvider starts a streaming completion
    Then the stream yields one Err(ProviderError::Auth) and no TextDelta chunks

  Scenario: build_stream_request produces streaming body
    Given a Rhai script whose build_stream_request clones build_request and sets "stream": true
    When the provider invokes build_stream_request with a user message
    Then the returned JSON body has stream equal to true

  Scenario: End-to-end stream against mock SSE server
    Given a wiremock SSE endpoint returning three content deltas and one stop event
    When RhaiCustomProvider performs a streaming completion
    Then the collected chunks are TextDelta, TextDelta, TextDelta, StopReason in that exact order
