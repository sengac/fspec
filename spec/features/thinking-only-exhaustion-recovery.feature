@done
@BUG-221
@cli
@providers
@streaming
@provider-abstraction
@thinking-exhaustion
Feature: Recover thinking-only token exhaustion without numerical reasoning usage
  """
  Captured reasoning and visible answer progress are separate from provider billing usage.
  The provider-agnostic stream finalization must recognize thinking-only length stops
  even when reasoning_tokens is absent and output_tokens includes all reasoning.
  Reuse the existing two-retry exhaustion recovery, preserve reasoning context and
  leave numerical usage and ordinary auto-continue length-stop safety unchanged.
  BUG-222 owns request-setting changes; Gemini-specific continuation is separate.
  """

  Scenario: Thinking-only length stop recovers without numerical reasoning usage
    Given a streamed thinking-only response with absent reasoning usage and 4096 output tokens
    When the response stops for length and the next response provides an answer
    Then exactly one thinking exhaustion retry preserves the captured reasoning
    And provider usage is unchanged and no auto-continue nudge is consumed

  Scenario: Repeated thinking-only length stops exhaust the bounded recovery budget
    Given every streamed response contains only thinking with absent reasoning usage and 4096 output tokens
    When every response stops for length
    Then exactly two thinking exhaustion retries occur before completion
    And the exhaustion warning is visible and no auto-continue nudge is consumed

  Scenario: Useful answer progress and ordinary truncation do not trigger thinking-only recovery
    Given length-limited responses with useful mixed output or ordinary text without thinking
    When the responses finish streaming
    Then each response completes without a thinking exhaustion retry or auto-continue nudge

  Scenario: Non-length thinking-only completions do not enter exhaustion recovery
    Given thinking-only responses with clean absent or unsafe stop reasons
    When the responses finish with auto-continue disabled
    Then no thinking exhaustion retry occurs

  Scenario: Terminal provider errors do not trigger thinking-only recovery
    Given a provider rejects the streaming request with a terminal error
    When the stream loop handles the error with auto-continue armed
    Then no recovery request or auto-continue nudge is sent

  Scenario: User cancellation prevents a thinking-only recovery request
    Given a thinking-only length response whose thinking event triggers user cancellation
    When the stream loop observes the cancellation
    Then no recovery request or auto-continue nudge is sent

  Scenario: Tool progress prevents thinking-only recovery
    Given a length-limited response containing thinking and a completed tool call
    When the model stops without an answer after that tool progress
    Then no thinking exhaustion retry or auto-continue nudge is sent
