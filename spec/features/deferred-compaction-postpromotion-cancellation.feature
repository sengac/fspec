@BUG-214
@phase-1
@agent-core
@compaction
@context-management
Feature: Deferred compaction cancellation preserves reconciled inference readiness
  """
  Architecture notes:
  Exercise the real agent-loop caller after an inject_summary tool handoff.
  Interrupt synchronously when its committed context is published, after durable
  promotion but before deferred application returns. Reconciliation must finish
  and cancellation must suppress success and automatic continuation without
  installing a fresh inference block over the authoritative generation.
  """

  Scenario: Cancelling publication of a deferred summary keeps the session usable
    Given an agent task submits a valid summary through its deferred tool handoff
    When the user interrupts during publication of the promoted summary
    Then the promoted generation remains authoritative and reconciled
    And the agent-loop caller leaves inference ready for a fresh user task
    And no compaction success or automatic continuation is emitted
    And compaction activity is cleared and the original conversation remains accessible
