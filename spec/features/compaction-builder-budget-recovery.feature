@BUG-641
@done
@BUG-211
@phase-1
@agent-core
@context-management
@compaction
@BUG-304
Feature: Compaction builder budget recovery
  """
  The compactor has a bounded exploration budget and one tool-free synthesis
  opportunity using its actual gathered evidence. The normal DAG validator,
  selected request admission, durable commit, and publication remain mandatory.
  A fresh user input explicitly restarts only a failed pre-commit builder;
  synthetic continuations and unresolved publication failures cannot do so.
  """

  Scenario: Exhausted exploration can finish a summary without further tools
    Given a compactor uses its exploration budget reading persisted originals
    When its exploration ends before it returns a summary
    Then one tool-free synthesis request includes the gathered evidence
    And its valid summary is returned without another exploration round

  Scenario: Streaming synthesis preserves the final original evidence page
    Given a streaming compactor uses its exploration budget reading persisted originals
    When its final tool result contains evidence needed by the summary
    Then one tool-free streaming synthesis request includes that final tool result
    And the valid summary is returned on the same provider and model

  Scenario: Invalid synthesized summaries are never installed
    Given an exhausted compactor synthesizes an invalid summary
    When the bounded candidate correction budget is exhausted
    Then compaction fails without changing the durable conversation boundary

  Scenario: Provider failures do not trigger budget synthesis
    Given a compactor receives a provider error rather than a depth limit
    When the builder reports the failure
    Then no tool-free synthesis request is made

  Scenario: Fresh user input retries a failed builder before ordinary inference
    Given automatic compaction failed before committing a summary
    When a fresh user message arrives after the builder becomes available
    Then a new summary is committed before ordinary inference resumes
    And the user message completes without replaying the oversized request

  Scenario: Repeated builder failures wait for the next input
    Given automatic compaction failed before committing a summary
    When a fresh user message retries the still failing builder
    Then one builder attempt fails and the session returns idle
    And no ordinary inference or automatic retry starts

  Scenario: Unrelated inference blocks cannot restart compaction
    Given ordinary inference is blocked for an unresolved publication failure
    When a fresh user message arrives
    Then the existing failure remains blocked
    And neither a builder nor ordinary inference starts

  Scenario: Background notifications retry a failed builder
    Given automatic compaction failed before committing a summary
    When another agent sends a background notification after the builder becomes available
    Then the builder runs again with a fresh budget and commits a summary
    And the notification runs ordinary inference after the commit

  Scenario: A failed threshold compaction carries on uncompacted while the context fits
    Given the compaction threshold is below the usage and the usage fits the usable window
    When a user message triggers threshold compaction and the builder fails
    Then the UI warns Compaction failed with the exact builder error
    And the same turn answers from the uncompacted context and the session is not held

  Scenario: A failed in-turn threshold compaction carries on uncompacted while the context fits
    Given a turn starts below the compaction threshold
    When a tool round reports usage past the threshold that still fits and the builder fails
    Then the UI warns Compaction failed with the exact builder error
    And the turn continues uncompacted to its answer and the session is not held

  Scenario: A failed threshold compaction holds the turn when the context does not fit
    Given the usage is past the usable window
    When a user message triggers threshold compaction and the builder fails
    Then the turn ends with the overflow error naming the usage, the usable window and the builder error
    And no ordinary inference starts and the session is held

  Scenario: The next input retries a threshold compaction that carried on
    Given a failed threshold compaction carried on uncompacted
    When the next user message arrives after the builder becomes available
    Then a new summary is committed before ordinary inference
