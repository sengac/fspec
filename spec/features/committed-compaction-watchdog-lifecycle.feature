@agent-core
@context-management
@compaction
@CMPCT-057
Feature: Committed compaction watchdog lifecycle
  """
  Operation deadlines include executing builder work. Attempts belong to one
  operation and committed success resets them, independently of pending state.
  Failure retains blocked state; no synthetic Continue or forced pin is allowed.
  """

  Scenario: Committed success resets operation attempts
    Given a failed compaction operation followed by explicit resolution
    When the resolved operation commits successfully
    And another compaction operation begins
    Then its attempts start fresh

  Scenario: Executing work is bounded by its operation deadline
    Given a builder that never completes
    When its declared operation deadline expires
    Then a typed deadline failure is returned
    And no synthetic continuation or completion is emitted

  Scenario: Blocked work cannot implicitly start another builder
    Given a failed runtime compaction operation
    When automatic recovery invokes the operation again without explicit resolution
    Then the builder is not invoked again
    And the failure remains blocked without completion

  Scenario: Explicit resolution retains the operation deadline
    Given an operation whose builder fails before its deadline
    When explicit resolution retries the same operation after its deadline
    Then its identity is unchanged and the retry fails with a deadline outcome
