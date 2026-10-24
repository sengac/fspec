@phase-1
@agent-core
@agent-manager
@regression
Feature: Awaiting idle survives completion races and stream lag
  """
  Both AgentManager handlers share subscription ordering and completion waiting.
  Subscribe before inspecting status, and recheck authoritative status after lag.
  Waiting remains notification driven, cancellable and subject to the caller's timeout.
  This does not repair lost relay output or guarantee delivery of response attempt markers.
  """

  Scenario: Completion during initial status inspection is not missed
    Given a running subordinate with no timeout on its idle wait
    When it completes while the waiter inspects its initial status
    Then the waiter observes completion without requiring another event

  Scenario: Lagged completion resolves from authoritative idle status
    Given the subordinate's idle event has been overwritten by later output
    When the waiter detects stream lag and the subordinate is idle
    Then the wait resolves as idle without another state change

  Scenario: Lag while still running does not finish the wait
    Given stream lag while the subordinate is still running
    When the waiter recovers and the subordinate subsequently becomes idle
    Then the wait resolves only after completion

  Scenario: A destroyed subordinate resolves after stream lag
    Given stream lag and a subordinate that no longer exists
    When the waiter rechecks authoritative status
    Then the wait resolves as destroyed

  Scenario: An already idle subordinate needs no stream event
    Given a subordinate whose authoritative status is already idle
    When its idle wait subscribes and inspects status
    Then the waiter resolves immediately

  Scenario: A closed stream resolves as destroyed
    Given a running subordinate with a subscribed idle waiter
    When its stream closes before completion
    Then the wait resolves as destroyed
