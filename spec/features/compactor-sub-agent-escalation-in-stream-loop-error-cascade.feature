@done
@agent-core
@context-management
@compaction
@CMPCT-044
Feature: Compactor sub-agent escalation in the stream-loop error cascade

  """
  CMPCT-044 (cascade slice): when the API rejects a request with a
  context-overflow error the in-loop in-view compaction could not
  resolve, the stream-loop error cascade escalates to the compactor
  sub-agent. The cascade ordering is the contract: the robust
  `is_context_overflow_error` classifier runs BEFORE the NET-001
  transient-network retry arm (overflow 400s that abort the SSE stream
  must never be misclassified as network retries), AFTER the typed
  `classify_compaction_branch` (the PromptCancelled downcast stays the
  authoritative first signal), is gated on compactable turns
  (PROV-010), and the escalation shares the MAX_COMPACTION_RETRIES
  budget with the in-view rounds.
  """

  Background: User Story
    As a long-running agent session
    I want context-overflow API errors to trigger the compactor sub-agent escalation
    So that sessions compact and resume instead of dying or being network-retried against the same oversized payload

  @compaction
  @context-management
  @integration
  Scenario: Unmatched overflow error triggers compaction instead of terminating the session
    Given a streaming session with compactable turns whose context exceeds the provider limit
    And the provider returns a 400 whose body matches no existing is_prompt_too_long_error substring
    When the stream loop processes the error
    Then is_context_overflow_error classifies it as a context overflow
    And the compaction-recovery path runs for the session
    And the session does NOT terminate with a terminal "Agent error"

  @compaction
  @context-management
  @integration
  @regression
  Scenario: Overflow error is classified before the transient-network retry arm
    Given a provider context-overflow 400 that aborts the SSE stream mid-response
    And the error surface also matches a transient-network pattern such as "stream closed before completion"
    When the stream loop classifies the error
    Then the overflow branch is taken
    And the error is NOT retried as a transient network error
    And no "Reconnecting..." network-retry status is emitted for that error

  @compaction
  @context-management
  @integration
  Scenario: Overflow classifier defers to the typed PromptCancelled branch
    Given a stream error that carries a typed PromptError::PromptCancelled in its chain
    When the stream loop classifies the error
    Then classify_compaction_branch routes it to the compaction-cancel recovery path first
    And the context-overflow string classifier is not the deciding signal for that error

  @compaction
  @context-management
  @regression
  Scenario: Sub-agent escalation is bounded by the shared retry budget
    Given the in-loop in-view compaction retry budget has been exhausted for a turn
    And another context-overflow error occurs
    When the error cascade processes it
    Then the turn terminates with the structured compaction-budget-exhausted error
    And the compactor sub-agent is not spawned for a turn past the retry budget
