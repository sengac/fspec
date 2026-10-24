@done
@BUG-234
@phase-1
@agent-core
@context-management
@compaction
Feature: Compaction reserves elapsed time for synthesis and correction
  """
  One absolute operation deadline is inherited by registered recovery and direct
  generation. Candidate attempts share the remaining budget rather than reset
  it. Exploration uses a bounded fraction of remaining time before yielding to
  tool-free synthesis with captured evidence. Synthesis may borrow unused time
  up to the original operation deadline rather than fail at a soft estimate.
  Completed tools are captured as they finish, including within unfinished
  batches, preserving authentic tool identifiers and response scaffolding.
  Phase duration diagnostics contain identities and budgets, not payloads.
  Validation, cancellation and guarded durable publication remain mandatory.
  """

  Scenario: Slow exploration yields to synthesis on both compaction routes
    Given both compaction routes gather original evidence within a finite operation budget
    When exploration stalls after gathering evidence
    Then tool-free synthesis runs before the operation deadline
    And a valid complete summary is durably published

  Scenario: Stalled synthesis fails closed on the shared deadline
    Given a compactor has an existing durable generation
    When synthesis cannot finish within its reserved time
    Then the operation reports a phase-specific deadline failure
    And the durable generation remains unchanged without a completion event

  Scenario: Candidate correction inherits the remaining operation budget
    Given a compaction operation rejects its first candidate
    When bounded correction starts
    Then the next attempt retains the original absolute deadline
    And stalled work cannot reset or outlive that deadline

  Scenario: Completed evidence survives a timeout inside an unfinished tool batch
    Given streaming and nonstreaming compaction routes read original evidence before a slow command in the same response
    When exploration times out while the slow command is still unfinished
    Then synthesis retains the completed originals with their authentic assistant and tool pairing
    And accompanying assistant text and reasoning remain attached to the completed calls
    And the unfinished command is not presented as completed
    And only a validated complete summary is durably published

  Scenario: Synthesis can borrow unused time beyond a candidate estimate
    Given both compaction routes reserve synthesis time within one absolute operation deadline
    When valid synthesis exceeds its candidate estimate but fits the remaining operation time
    Then synthesis completes without another exploration attempt or a reset deadline
    And the validated summary is durably published before the operation expires

  Scenario: Expiry before durable publication rejects an otherwise valid candidate
    Given both compaction routes produce a valid candidate before the shared deadline
    When that deadline expires during admission or before deferred application
    Then no new durable generation is promoted and no completion event is emitted
    And the previously committed generation remains authoritative
