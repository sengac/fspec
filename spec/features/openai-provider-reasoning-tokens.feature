@done
@PROV-081
@providers
@rust
@rig
@bug-fix
@thinking-detection
@streaming
Feature: OpenAI provider drops vLLM-native reasoning/thinking tokens (streaming + non-streaming)
  """
  Streaming fix in rust/patches/rig-core/src/providers/openai/completion/streaming.rs:35-44. Simplest approach: add `#[serde(alias = "reasoning")]` to the existing `reasoning_content` field (line 41). Consumption at lines 297-303 stays unchanged. If we need both fields kept distinct for concatenation, add a second `reasoning: Option<String>` field and OR/concatenate at the consumption site instead.
  Non-streaming fix in rust/patches/rig-core/src/providers/openai/completion/mod.rs. Current state: Message::Assistant (lines 113-144) has no reasoning field; AssistantContent enum (lines 180-185) has only Text and Refusal. Required changes: (1) add `reasoning: Option<String>` with `#[serde(alias = "reasoning_content")]` (or both fields) to the Assistant variant, (2) propagate into the TryFrom<CompletionResponse> extraction at lines 757-791 by emitting a new `completion::AssistantContent::reasoning(...)` entry (or extend CompletionResponse with a reasoning field). The outbound panic at line 541 for AssistantContent::Reasoning stays — it guards against serializing reasoning back out to the OpenAI request.
  Request-side guard: audit the outgoing CompletionRequest builder in completion/mod.rs (`TryFrom<OpenAIRequestParams> for CompletionRequest` around lines 1005-1093) to ensure neither `include_reasoning` nor `chat_template_kwargs.enable_thinking` is emitted. If additional_params is user-passthrough, document that callers should not set these keys; tests should assert the default-built request body does not contain them.
  Server-side evidence (unchanged, for reviewer context): vLLM DeltaMessage.reasoning at vllm/entrypoints/openai/engine/protocol.py:258-262; ChatMessage.reasoning at chat_completion/protocol.py:54-64; Qwen3ReasoningParser enable_thinking default True; include_reasoning default True. No vLLM server-side changes needed for this work unit. Local launcher at $HOME/vllm.sh already passes --reasoning-parser qwen3 (line 408).
  Tests run against Rust code under rust/patches/rig-core via `cargo test` in that crate. Prefer small unit tests that deserialize canned JSON payloads (both vLLM and GLM shapes) and assert the deserialized value exposes the reasoning text. Streaming path test: feed lines through the streaming decoder and collect emitted RawStreamingChoice variants. Non-streaming path test: deserialize a full CompletionResponse fixture and assert CompletionResponse<_> surfaces reasoning.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Streaming deserialization must accept both `reasoning` (vLLM) and `reasoning_content` (Z.AI/GLM) on the delta object and surface whichever arrives as a ReasoningDelta to the downstream stream.
  #   2. Non-streaming assistant message must capture reasoning text from either `reasoning` or `reasoning_content` on the message object and expose it to callers (today neither field is captured — the extraction in TryFrom<CompletionResponse> only emits Text and Refusal).
  #   3. If both `reasoning` and `reasoning_content` arrive on the same payload, their content must be concatenated (reasoning_content first, then reasoning) so nothing is silently lost.
  #   4. Absence of both reasoning fields must not break normal content streaming or non-streaming responses (no regression on providers that don't emit reasoning).
  #   5. Outgoing OpenAI Chat Completion request bodies must NOT contain `include_reasoning: false` (vLLM strips reasoning when this is false; default true is correct).
  #   6. Outgoing OpenAI Chat Completion request bodies must NOT contain `chat_template_kwargs.enable_thinking: false` on Qwen3 requests (this is the template-level kill-switch that suppresses <think> generation at the model level).
  #
  # EXAMPLES:
  #   1. Streaming chunk with only {delta:{content:'hello'}} and no reasoning fields streams through unchanged — no ReasoningDelta emitted, no error.
  #   2. Non-streaming response {choices:[{message:{role:'assistant',reasoning:'analysis',content:'answer'}}]} deserializes successfully AND the caller-visible CompletionResponse exposes reasoning text 'analysis' alongside content 'answer'.
  #   3. Non-streaming response {message:{role:'assistant',reasoning_content:'analysis',content:'answer'}} (Z.AI shape) also surfaces reasoning text 'analysis' to the caller — both field names work.
  #   4. Non-streaming response with {message:{role:'assistant',content:'answer'}} (no reasoning fields) deserializes and works exactly as before — no reasoning surfaced, no error.
  #   5. Outgoing request body serialized from a caller request contains NO `include_reasoning` key and NO `chat_template_kwargs.enable_thinking` key (assert on the JSON payload sent to /chat/completions).
  #   6. A user streams a chat completion from their local vLLM Qwen3 server and sees thinking tokens arrive in the reasoning channel before the final answer arrives in the content channel (previously: thinking tokens vanished silently).
  #   7. A user pointed at a Z.AI/GLM endpoint still sees reasoning exactly as before — switching field names between servers is transparent, no regression.
  #   8. A user talks to a hypothetical proxy that sends both field names at once; they see all of the reasoning concatenated, nothing is dropped.
  #   9. A user talks to a plain OpenAI endpoint that emits no reasoning at all; their content streams exactly as before — no error, no empty reasoning noise.
  #   10. A user issues a NON-streaming request to vLLM Qwen3 and the CompletionResponse they receive carries both the final answer content and the reasoning text (previously: only content returned).
  #   11. A user issues a non-streaming request to a Z.AI endpoint and receives reasoning text via the `reasoning_content` field shape — same caller-visible surface.
  #   12. A user inspects the HTTP request body sent for any Chat Completion and sees NO `include_reasoning: false` key and NO `chat_template_kwargs.enable_thinking: false` key (the client never suppresses server-side reasoning).
  #
  # ========================================
  Background: User Story
    As a fspec user running a local vLLM server with a Qwen3 reasoning model
    I want to see the model's thinking/reasoning tokens appear in my streaming and non-streaming responses
    So that I can debug prompts, evaluate reasoning quality, and get the same thinking visibility I already have with Z.AI/GLM providers

  @unit
  @rust
  Scenario: Streaming surfaces vLLM-native `reasoning` field as reasoning output
    Given a caller streams a chat completion through the OpenAI provider
    And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning":"Let me analyse..."}}]}
    When the provider decodes the chunk
    Then the caller receives a ReasoningDelta whose reasoning text equals "Let me analyse..."
    And the caller does not receive any content delta for that chunk
    And the chunk does not produce a decode error

  @unit
  @rust
  @backward-compatibility
  Scenario: Streaming still surfaces Z.AI/GLM `reasoning_content` field as reasoning output
    Given a caller streams a chat completion through the OpenAI provider
    And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}
    When the provider decodes the chunk
    Then the caller receives a ReasoningDelta whose reasoning text equals "thinking..."
    And the caller does not receive any content delta for that chunk

  @unit
  @rust
  Scenario: Streaming concatenates reasoning with reasoning_content first then reasoning when both fields are present
    Given a caller streams a chat completion through the OpenAI provider
    And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"reasoning_content":"B","reasoning":"A"}}]}
    When the provider decodes the chunk
    Then the caller receives reasoning text equal to "BA" with reasoning_content concatenated first and reasoning appended
    And no reasoning character from either source field is dropped

  @unit
  @rust
  @regression
  Scenario: Streaming passes through normal content chunks unchanged when no reasoning fields are present
    Given a caller streams a chat completion through the OpenAI provider
    And the upstream server emits a streaming chunk with body {"choices":[{"delta":{"content":"hello"}}]}
    When the provider decodes the chunk
    Then the caller receives a content delta whose text equals "hello"
    And the caller does not receive any ReasoningDelta
    And the chunk does not produce a decode error

  @unit
  @rust
  Scenario: Non-streaming surfaces vLLM-native `reasoning` field on the assistant message
    Given a caller issues a non-streaming chat completion through the OpenAI provider
    And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning":"analysis","content":"answer"}
    When the provider decodes the response
    Then the caller-visible CompletionResponse exposes reasoning text "analysis"
    And the caller-visible CompletionResponse exposes content text "answer"

  @unit
  @rust
  @backward-compatibility
  Scenario: Non-streaming surfaces Z.AI/GLM `reasoning_content` field on the assistant message
    Given a caller issues a non-streaming chat completion through the OpenAI provider
    And the upstream server returns a response whose assistant message is {"role":"assistant","reasoning_content":"analysis","content":"answer"}
    When the provider decodes the response
    Then the caller-visible CompletionResponse exposes reasoning text "analysis"
    And the caller-visible CompletionResponse exposes content text "answer"

  @unit
  @rust
  @regression
  Scenario: Non-streaming works unchanged when the assistant message has no reasoning field
    Given a caller issues a non-streaming chat completion through the OpenAI provider
    And the upstream server returns a response whose assistant message is {"role":"assistant","content":"answer"}
    When the provider decodes the response
    Then the caller-visible CompletionResponse exposes content text "answer"
    And the caller-visible CompletionResponse does not expose any reasoning text
    And the response does not produce a decode error

  @unit
  @rust
  Scenario: Outgoing request body never contains reasoning-suppression keys
    Given a caller builds a chat completion request through the OpenAI provider without explicitly setting any reasoning-suppression flag
    When the provider serializes the request body that would be POSTed to /chat/completions
    Then the serialized body does not contain a top-level key named "include_reasoning"
    And the serialized body does not contain a "chat_template_kwargs.enable_thinking" key path
