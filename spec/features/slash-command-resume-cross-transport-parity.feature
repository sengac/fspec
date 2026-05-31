@done
@RPC-049
@session-management
@rust
@multi-session
@rpc
@agent-view
@tui
@slash-command
Feature: /resume cross-transport parity
  """
  RPC-049 split-out feature file. Validates that the new `resume_session`
  aggregate RPC round-trips identically across EmbeddedFspecBackend AND
  WebSocketFspecBackend against the SAME StubSessionManagerHandle.

  Mirrors the RPC-037 cross-transport parity pattern in
  `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs`.

  The StubSessionManagerHandle's `resume_session_calls()` accessor is
  used to assert byte-equal call counts (one per transport).
  """

  Background: User Story
    As a fspec engineer wiring the /resume durable restore RPC
    I want resume_session to round-trip identically across both transports
    So that the AgentView behaves identically whether the daemon runs embedded or over WebSocket

  Scenario: resume_session round-trips identically across both transports
    Given a SharedFspecService wired to a StubSessionManagerHandle
    And an EmbeddedFspecBackend over that service
    And a WebSocketFspecBackend over that same service
    When backend.resume_session(SessionId("stub-1")) is awaited through each transport
    Then both calls return Ok(())
    And the StubSessionManagerHandle's resume_session call counter increments by 2 (once per transport)
