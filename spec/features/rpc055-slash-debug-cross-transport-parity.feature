@done
@testing
@RPC-055
@rpc
@rust
Feature: /debug debug-capture cross-transport parity

  """
  Both EmbeddedFspecBackend (in-process embedded transport) and
  WebSocketFspecBackend (tarpc over WebSocket) must land identically on
  the same StubSessionManagerHandle for the new set_debug_directory RPC
  method and the existing toggle_debug RPC method. Mirrors the RPC-049 /
  RPC-050 / RPC-054 cross-transport parity tests — each transport
  invocation increments the same per-stub counter.
  """

  Background: User Story
    As a developer porting the AgentView to Rust
    I want both transports to land identically on the SessionManagerHandle for /debug RPCs
    So that the WebSocket and embedded paths cannot diverge as the feature grows

  Scenario: Embedded and WebSocket set_debug_directory both reach the stub
    Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When set_debug_directory("/tmp/dbg-A") is called via the embedded transport
    And set_debug_directory("/tmp/dbg-B") is called via the WebSocket transport
    Then the stub's set_debug_directory_calls counter equals 2
    And both calls return Ok(())

  Scenario: Embedded and WebSocket toggle_debug both reach the stub
    Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    And a session s-1 has been created on the stub
    When toggle_debug(s-1, "/tmp/dbg-A") is called via the embedded transport
    And toggle_debug(s-1, "/tmp/dbg-B") is called via the WebSocket transport
    Then the stub's toggle_debug_calls counter equals 2
    And both calls return Ok with a non-empty path string
