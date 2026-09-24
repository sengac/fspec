@done
@agent-core
@context-management
@compaction
@CMPCT-044
Feature: Context-overflow error classifier

  """
  CMPCT-044 (classifier slice): the stream-loop error cascade only
  triggered compaction when the API error matched the fixed
  `is_prompt_too_long_error` substring list (or a typed
  `PromptCancelled`). Provider context-overflow errors with different
  wording (or wrapped in error chains) fell through to the terminal
  error arm and killed the session. This feature specifies the new
  robust `is_context_overflow_error` classifier — a strict superset of
  `is_prompt_too_long_error` that walks the full anyhow error chain and
  keeps the PROV-010 thinking-budget exclusion.
  """

  Background: User Story
    As a long-running agent session
    I want provider context-overflow errors to be detected reliably
    So that the clean compaction sub-agent can be triggered instead of the session dying

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier matches every error is_prompt_too_long_error matches
    Given a provider context-overflow error string that the existing is_prompt_too_long_error classifier matches
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true
    And the new classifier matches it for the same reason as the old one

  @compaction
  @context-management
  Scenario: Overflow classifier matches provider-variant wording the old classifier misses
    Given an OpenAI-compatible provider error "Input is too long: 201,000 tokens > 200,000 maximum"
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true
    And is_prompt_too_long_error returns false for that same string

  @compaction
  @context-management
  Scenario: Overflow classifier matches Bedrock and Vertex context-limit wording
    Given a provider error in the style "The input (approximately 200,500 tokens) exceeds the maximum number of input tokens (200,000)"
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns true

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier rejects thinking-budget configuration errors
    Given an API error containing "budget_tokens" from a thinking-budget configuration failure
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns false

  @compaction
  @context-management
  @regression
  Scenario: Overflow classifier rejects truncation and unrelated errors
    Given an API error that is NOT a context-overflow error (a truncation, rate-limit, or auth error)
    When is_context_overflow_error is called with that error string
    Then is_context_overflow_error returns false

  @compaction
  @context-management
  Scenario: Overflow classifier sees through anyhow context wrapping
    Given a context-overflow API error that is wrapped with anyhow context layers
    When is_context_overflow_error is called with the wrapped error
    Then is_context_overflow_error returns true based on the full error chain
