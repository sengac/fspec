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

  BUG-306: superseded in part by spec/features/structured-terminal-overflow-classification.feature
  (CMPCT-054) - runtime overflow recovery is classified only from machine-readable
  provider rejection codes, so the free-text matching scenarios were removed.
  """

  Background: User Story
    As a long-running agent session
    I want provider context-overflow errors to be detected reliably
    So that the clean compaction sub-agent can be triggered instead of the session dying

  @compaction
  @context-management
  @regression
  @compaction
  @context-management
  @compaction
  @context-management
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
