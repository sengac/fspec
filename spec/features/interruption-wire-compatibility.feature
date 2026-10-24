@done
@phase-1
@rpc
@session-management
@BUG-220
Feature: Interruption wire compatibility
  """
  Interrupted remains the unchanged legacy binary and JSON wire variant. A typed interruption variant is appended to the wire enum to avoid changing old bincode layouts or variant indices. Embedded and WebSocket broadcasts share the same serialized payload; manual bridge serializers publish the existing interrupted event type with an additional reason.
  """

  Background: User Story
    As a session client
    I want interruption provenance across supported transports
    So that all frontends can distinguish user stops and internal aborts

  Scenario: Typed and legacy interruption events retain their meaning
    Given interruption JSON with user or loopDetector reasons and an older reasonless event
    When each event is decoded and encoded again
    Then explicit causes and queued inputs survive while missing causes remain unspecified

  Scenario: Embedded and WebSocket subscribers receive identical typed interruptions
    Given embedded and WebSocket clients sharing a session broadcast
    When a typed interruption is published on the session channel
    Then both subscribers receive the same reason and queued inputs
