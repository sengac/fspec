@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex NAPI session recovery
  """
  NAPI dispatch binds each provider to its owning session before building the agent. The provider adapter shares the Codex recovery handle with the generation watchdog, and the existing session observer routes lifecycle notifications without coupling another session. Provider switching initializes any missing session extension.
  """

  Background: User Story
    As a NAPI Codex session owner
    I want network recovery to use my session's watchdog and status output
    So that a recoverable outage remains visible and cancellable

  Scenario: NAPI dispatch shares recovery state with its owning session
    Given two NAPI sessions dispatching Codex after starting with another provider
    When one session waits for an unavailable network beyond its generation stall deadline
    Then its watchdog remains pending and only its session emits reconnect status
    And cancelling the provider stream clears its recovery wait without changing the other session

  Scenario: NAPI dispatch clears routing state from the previous turn
    Given a NAPI Codex session has routing state from a completed turn
    When the next user turn prepares its provider
    Then the new request retains session affinity without the previous turn state
    And a late response from the previous turn cannot restore that turn state
