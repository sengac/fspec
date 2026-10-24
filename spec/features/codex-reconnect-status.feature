@done
@phase-1
@provider-abstraction
@providers
@PROV-160
@critical
Feature: Codex reconnect status
  """
  Session-owned Codex recovery handles publish reconnecting, recovered, failed and cancelled lifecycle notifications through the existing provider observer and StreamEvent Status pipeline. Provider outages never impersonate daemon disconnection or invoke generic Continue recovery.
  """

  Background: User Story
    As a Codex session owner
    I want to see the reconnect lifecycle for my session
    So that network waiting is visible without affecting other sessions

  Scenario: Reconnect lifecycle is session scoped and reaches existing status output
    Given two Codex sessions using the existing provider status observer
    When one session reconnects recovers exhausts or is cancelled
    Then only that session emits reconnect lifecycle status without daemon disconnect events or generic Continue recovery
