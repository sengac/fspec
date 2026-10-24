@PROV-155
@providers
@integration
@provider-abstraction
Feature: Codex session cache routing
  """
  Runtime sessions own routing independently from providers reconstructed each turn.
  User turns clear server state, while history replacement rotates affinity.
  Restored and subordinate sessions create independent routing handles.
  """

  Scenario: Session routing persists across provider reconstruction and resets with history
    Given a session with Codex routing and a server turn state
    When a new turn starts and the conversation is replaced
    Then the new turn keeps affinity without turn state and replacement rotates affinity
    And another session and an ordinary provider share no routing
