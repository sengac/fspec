@done
@providers
@codelet
@bug-fix
@streaming
@PROV-039
Feature: stop_reason lost in streaming — output truncation silently treated as normal completion
  """
  See vtcode's 6-layer truncated tool call recovery pipeline as reference: ToolCallBuilder → parse_tool_arguments with extract_balanced_json → PreparedAssistantToolCall with error recording → dispatch error responses → turn-level recovery → post-tool LLM failure recovery
  Key vtcode code references: vtcode-commons/src/llm.rs (FinishReason enum), ResponseAggregator (streaming accumulation), map_finish_reason_common() (provider normalization), ContinuationController (task-level continuation), thinking signature-gated round-tripping for truncated thinking blocks
  Affected files in our codebase: rust/patches/rig-core/src/providers/anthropic/streaming.rs:346-374 (stop_reason discarded), rust/patches/rig-core/src/agent/prompt_request/streaming.rs:185-188 (FinalResponse missing field), rust/cli/src/interactive/stream_loop.rs (zero stop_reason checks), rust/napi/src/persistence/message_envelope.rs:299,329,537,568 (hardcoded end_turn)
  Secondary bug: ProviderManager::max_output_tokens() returns compile-time constant for OpenAI (4096) instead of runtime OPENAI_MAX_OUTPUT_TOKENS env var value — compaction threshold miscalculation
  Follow vtcode's pattern: stop_reason propagates as metadata through the entire pipeline, while control flow decisions are based on structural presence of tool calls/content — not stop_reason branching
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. stop_reason must be propagated from the SSE stream through FinalResponse to the agent loop — never discarded
  #   2. Each provider (Anthropic, OpenAI, Gemini) must normalize its wire-format stop reason into the codelet StopReason enum
  #   3. Persistence must store the real stop_reason — never hardcode end_turn
  #   4. When stop_reason is max_tokens and no tool calls were made, a user-visible truncation warning must be shown in the TUI
  #   5. Truncated tool calls must produce an informative error message identifying the truncation as the cause — not just generic JSON parse failure
  #   6. OpenAI max_output_tokens must read runtime env var value, not compile-time constant
  #
  # EXAMPLES:
  #   1. Anthropic sends message_delta with stop_reason='max_tokens' → FinalResponse contains StopReason::MaxTokens → stream_loop displays truncation warning
  #   2. Model hits max_tokens mid-tool-call → JSON parse fails → error message says 'Tool call truncated due to output token limit' instead of generic JSON error → model retries
  #   3. Model completes normally with end_turn → no warning shown, stop_reason persisted as 'end_turn' — baseline behavior unchanged
  #   4. OpenAI model hits max_tokens → persisted stop_reason is 'max_tokens', not 'end_turn'
  #   5. OPENAI_MAX_OUTPUT_TOKENS env var set to 16384 → ProviderManager::max_output_tokens() returns 16384, not hardcoded 4096
  #
  # ========================================
  Background: User Story
    As a developer using the AI agent
    I want to see when the LLM response was truncated due to max_tokens
    So that I know the response is incomplete and can ask the model to continue

  @streaming
  @anthropic
  Scenario: Anthropic streaming propagates max_tokens stop_reason through FinalResponse
    Given the agent is using the Anthropic provider in streaming mode
    And the model hits the max_tokens limit during text generation
    When the Anthropic SSE stream emits a message_delta with stop_reason "max_tokens"
    Then the FinalResponse yielded from the streaming pipeline contains StopReason::MaxTokens
    And the stream_loop displays a truncation warning to the user
    And the persisted AssistantMessage stop_reason is "max_tokens"

  @streaming
  @tool-calls
  Scenario: Truncated tool calls produce informative error identifying truncation as the cause
    Given the agent is using any provider in streaming mode
    And the model hits max_tokens while generating a tool call JSON body
    When the accumulated tool call arguments fail JSON parsing due to truncation
    Then the error message sent back to the model contains "Tool call truncated due to output token limit"
    And the error message does not contain only a generic JSON parse failure
    And the agent loop continues to allow the model to retry

  @streaming
  @baseline
  Scenario: Normal end_turn completion has no truncation warning and correct persistence
    Given the agent is using any provider in streaming mode
    And the model completes its response naturally with stop_reason "end_turn"
    When the FinalResponse is yielded from the streaming pipeline
    Then no truncation warning is shown to the user
    And the persisted AssistantMessage stop_reason is "end_turn"

  @streaming
  @openai
  Scenario: OpenAI streaming propagates max_tokens stop_reason instead of hardcoding end_turn
    Given the agent is using the OpenAI provider in streaming mode
    And the model hits the max_tokens limit during text generation
    When the OpenAI SSE stream emits a response with finish_reason "length"
    Then the FinalResponse contains StopReason::MaxTokens
    And the persisted AssistantMessage stop_reason is "max_tokens"
    And the stop_reason is not hardcoded to "end_turn"

  @configuration
  @openai
  Scenario: OpenAI max_output_tokens reads runtime environment variable
    Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to "16384"
    When ProviderManager::max_output_tokens() is called for the OpenAI provider
    Then the returned value is 16384
    And the returned value is not the compile-time constant 4096
