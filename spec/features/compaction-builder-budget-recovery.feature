@done
@BUG-211
@phase-1
@agent-core
@context-management
@compaction
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

  Scenario: Repeated builder failures wait for fresh user input
    Given automatic compaction failed before committing a summary
    When a fresh user message retries the still failing builder
    Then one builder attempt fails and the session returns idle
    And no ordinary inference or automatic retry starts

  Scenario: Unrelated inference blocks cannot restart compaction
    Given ordinary inference is blocked for an unresolved publication failure
    When a fresh user message arrives
    Then the existing failure remains blocked
    And neither a builder nor ordinary inference starts

  Scenario: Background notifications cannot authorize a failed builder retry
    Given automatic compaction failed before committing a summary
    When another agent sends a background notification
    Then the failed operation remains blocked
    And neither a builder nor ordinary inference starts
