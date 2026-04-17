@done
@PROV-040
Feature: Truncated tool call recovery — auto-chunk large writes and retry on max_tokens
  """
  Recovery logic goes in stream_loop.rs at the point where truncated tool call errors are received — after the PROV-039 enriched error is emitted from the provider layer but before it's sent back to the model.
  Key detection point: the error message from PROV-039 contains 'Tool call truncated due to output token limit' — use this as the reliable signal to trigger recovery.
  The truncation counter should be a turn-level variable in stream_loop (reset each new user prompt).
  The recovery prompt is injected as a user message with structured guidance telling the model to use alternative strategies for large content.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a tool call is truncated (max_tokens + JSON parse failure), the error sent back to the model MUST include a structured recovery instruction — not just the raw error
  #   2. The recovery instruction must tell the model to use an alternative strategy: use Bash with heredoc/echo, split into multiple smaller Write calls, or use Write+Edit append pattern
  #   3. A truncation retry budget (max 2 consecutive truncation errors per turn) must prevent infinite retry loops — after budget exhausted, report failure to user
  #   4. Recovery logic must live in the stream loop (provider-agnostic) — not in provider-specific streaming code — so it works identically for Anthropic, OpenAI, and Gemini
  #   5. The recovery prompt must include the truncated tool name and partial arguments so the model knows exactly what failed and can reformulate
  #   6. Normal end_turn completions and non-truncation errors must be completely unaffected by the recovery logic
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to write large files without hitting an infinite truncation error loop
    So that I can complete file operations reliably regardless of content size

  @truncation-recovery
  Scenario: Truncated tool call error includes structured recovery instruction
    Given the agent is streaming a response from any provider
    And the model attempts a Write tool call with content exceeding the output token limit
    When the tool call is truncated due to max_tokens
    Then the error message contains a structured recovery instruction
    And the recovery instruction suggests using Bash with heredoc for large files
    And the recovery instruction suggests splitting into multiple smaller Write calls
    And the recovery instruction includes the truncated tool name
    And the recovery instruction includes the partial arguments that were received

  @truncation-recovery
  Scenario: Retry budget prevents infinite truncation retry loops
    Given the agent is streaming a response from any provider
    And the truncation retry budget is set to 2
    And the model has already exhausted the retry budget with consecutive truncation errors
    When the budget-exhausted error is generated
    Then the error message includes the retry count and informs the user the budget is exhausted
    And the error message suggests alternative strategies for large content
    And the stream loop terminates without starting another retry

  @truncation-recovery
  Scenario: Normal completion is unaffected by truncation recovery logic
    Given the agent is streaming a response from any provider
    And the model completes a tool call normally with stop_reason end_turn
    When the stream completes
    Then no recovery instruction is injected
    And the truncation retry counter remains at zero
    And the behavior is identical to pre-PROV-040 baseline

  @truncation-recovery
  Scenario: Text-only truncation does not trigger tool call recovery
    Given the agent is streaming a response from any provider
    And the model hits max_tokens during a text-only response with no tool call
    When the stream completes with stop_reason max_tokens
    Then the existing PROV-039 truncation warning is displayed
    And no tool call recovery instruction is injected

  @truncation-recovery
  Scenario: Truncation recovery is provider-agnostic
    Given the truncation detection relies on the error message string from PROV-039
    When a truncation error containing "Tool call truncated due to output token limit" is received
    Then the same recovery logic fires regardless of whether the provider is Anthropic, OpenAI, or Gemini
    And the recovery instruction content is identical across all providers
