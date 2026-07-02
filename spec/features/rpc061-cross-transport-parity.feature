@done
@RPC-061
@rust
@tui
@rpc
@supervisor
@session-management
@parity
Feature: RPC-061 cross-transport parity — supervisor surface
  """
  Cross-transport parity for the supervisor surface introduced by
  RPC-061. Drives identical scripted scenarios against
  EmbeddedFspecBackend AND WebSocketFspecBackend, both constructed
  against the SAME deterministic StubSessionManagerHandle, so both
  transports observe identical state and the stub records the right
  per-method call counts.

  Companion features:
  - spec/features/rpc061-source-shape.feature
  - spec/features/rpc061-supervisor-links.feature
  """

  Background: User Story
    As a fspec TUI maintainer adding a new SessionManagerHandle method
    I want both embedded and WebSocket transports to round-trip the call to the same stub state
    So that the AgentView behaves identically regardless of which transport is live

  Scenario: Embedded and WebSocket add_supervisor both reach the stub
    Given a fresh StubSessionManagerHandle behind both transports
    When add_supervisor is called via the embedded transport with subordinate=SessionId("sub-em") and supervisor=SessionId("sup")
    And add_supervisor is called via the WebSocket transport with subordinate=SessionId("sub-ws") and supervisor=SessionId("sup")
    Then the stub's add_supervisor_calls counter increased by 2
    And the stub now reports two subordinates for "sup"

  Scenario: Embedded and WebSocket get_supervisors return identical lists
    Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    When get_supervisors is called via the embedded transport
    And get_supervisors is called via the WebSocket transport
    Then both calls return [SessionId("sup")]
    And the stub's get_supervisors_calls counter increased by 2

  Scenario: Embedded and WebSocket get_subordinates return identical lists
    Given a StubSessionManagerHandle seeded with two subordinates of "sup"
    When get_subordinates is called via the embedded transport
    And get_subordinates is called via the WebSocket transport
    Then both calls return [SessionId("sub-a"), SessionId("sub-b")] (same order)

  Scenario: Embedded and WebSocket get_subordinate return identical results
    Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    When get_subordinate is called via each transport
    Then both calls return Some(SessionId("sub"))

  Scenario: Embedded and WebSocket receive_incoming_message both reach the stub
    Given a fresh StubSessionManagerHandle behind both transports
    When receive_incoming_message is called via the embedded transport
    And receive_incoming_message is called via the WebSocket transport
    Then the stub's receive_incoming_message_calls counter increased by 2
    And the stub's recorded_incoming_messages contains both payloads

  Scenario: Embedded and WebSocket remove_supervisor clear state
    Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    When remove_supervisor is called via the embedded transport
    Then the stub now reports no subordinates for "sup"
    When the call is repeated via the WebSocket transport (idempotent)
    Then both transports landed exactly one call each on the stub

  Scenario: Circular add_supervisor is rejected identically across transports
    Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    When add_supervisor(sup, sub) is attempted via the embedded transport
    Then it returns Err with message "circular supervision not allowed"
    And the same call via WebSocket returns the same error
