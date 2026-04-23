@done
@thinking
@rust
@validation
@validator
@streaming
@providers
@PROV-089
Feature: Add StreamChunk::ReasoningDelta variant and stream_convert plumbing for thinking tokens
  """
  ReasoningDelta is added as a distinct StreamChunk variant rather than reusing TextDelta so downstream consumers (stream_loop.rs) can later route it to StreamedAssistantContent::ReasoningDelta in the same MultiTurnStreamItem shape as the hardcoded ClaudeProvider.
  Wiring the StreamChunk::ReasoningDelta through to MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ReasoningDelta) in stream_loop.rs/gemini_continuation.rs is OUT OF SCOPE for this work unit — PROV-089 only introduces the variant plus stream_convert dispatch and a pass-through test through stream_http/provider_stream. A follow-up work unit will connect it to rig MultiTurnStreamItem.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. StreamChunk enum in codelet/providers/src/custom/stream.rs gains a ReasoningDelta(String) variant carrying the reasoning text fragment
  #   2. stream_convert::handle_one dispatches kind values 'reasoning_delta' and 'thinking_delta' to emit a StreamChunk::ReasoningDelta carrying the map's 'text' field
  #   3. A parse_stream_chunk map of kind 'reasoning_delta' with an empty or missing text field produces zero chunks (same behaviour as empty text_delta)
  #   4. ReasoningDelta chunks flow through the same stream_http::open_stream SSE adapter and end up in the public Stream<Item = Result<StreamChunk, ProviderError>> yielded by RhaiCustomProvider::complete_with_tools_streaming
  #   5. The ReasoningDelta variant is ordering-preserving: text and reasoning deltas interleaved on the wire are yielded in wire order
  #
  # EXAMPLES:
  #   1. parse_stream_chunk returns #{ kind: 'reasoning_delta', text: 'Let me think' } → bridge yields StreamChunk::ReasoningDelta("Let me think")
  #   2. parse_stream_chunk returns #{ kind: 'thinking_delta', text: 'computing...' } → bridge yields StreamChunk::ReasoningDelta("computing...")
  #   3. parse_stream_chunk returns #{ kind: 'reasoning_delta', text: '' } → bridge yields no chunk (ignored like empty text_delta)
  #   4. parse_stream_chunk returns #{ kind: 'reasoning_delta' } (no text field) → bridge yields no chunk
  #   5. Script emits ReasoningDelta('Hel') then TextDelta('answer') then ReasoningDelta('thinking done') → stream yields chunks in that exact order
  #   6. End-to-end: wiremock SSE server replaying a Claude-style SSE sequence with content_block_delta of type 'thinking_delta' → RhaiCustomProvider::complete_with_tools_streaming yields StreamChunk::ReasoningDelta items
  #
  # ========================================
  Background: User Story
    As a custom provider script author
    I want to emit reasoning_delta / thinking_delta chunks from parse_stream_chunk
    So that Rhai providers can stream Claude-style thinking tokens through the custom provider bridge at parity with the hardcoded ClaudeProvider

  Scenario: Emit ReasoningDelta chunk for a reasoning_delta kind
    Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta", text: "Let me think" }
    When the bridge feeds any SSE data payload through process_event
    Then the bridge yields one StreamChunk::ReasoningDelta with value "Let me think"

  Scenario: Accept thinking_delta kind as an alias for reasoning_delta
    Given a Rhai script whose parse_stream_chunk returns #{ kind: "thinking_delta", text: "computing..." }
    When the bridge feeds any SSE data payload through process_event
    Then the bridge yields one StreamChunk::ReasoningDelta with value "computing..."

  Scenario: Skip reasoning_delta with empty text
    Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta", text: "" }
    When the bridge feeds any SSE data payload through process_event
    Then no StreamChunk is yielded for that event

  Scenario: Skip reasoning_delta with missing text field
    Given a Rhai script whose parse_stream_chunk returns #{ kind: "reasoning_delta" } with no text field
    When the bridge feeds any SSE data payload through process_event
    Then no StreamChunk is yielded for that event

  Scenario: Preserve wire order for interleaved reasoning and text deltas
    Given a Rhai script that returns reasoning_delta or text_delta based on a marker in the event payload
    When the SSE stream emits a reasoning event then a text event then another reasoning event
    Then the bridge yields ReasoningDelta("Hel") followed by TextDelta("answer") followed by ReasoningDelta("thinking done")

  Scenario: End-to-end stream through complete_with_tools_streaming yields ReasoningDelta
    Given a wiremock SSE endpoint returning two thinking deltas followed by one text delta and a stop event
    When RhaiCustomProvider performs a streaming completion
    Then the collected chunks are ReasoningDelta, ReasoningDelta, TextDelta, StopReason in that exact order
