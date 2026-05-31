@done
@testing
@tui
@RPC-056
@rpc
@rust
Feature: /blocklist cross-transport parity
  """
  Both EmbeddedFspecBackend (in-process embedded transport) and
  WebSocketFspecBackend (tarpc over WebSocket) must land identically on
  the same StubSessionManagerHandle for the new blocklist_list RPC
  method. Mirrors the RPC-049 / RPC-050 / RPC-054 / RPC-055
  cross-transport parity tests — each transport invocation increments
  the same per-stub counter and returns the same payload.
  """

  Background: User Story
    As a developer porting the AgentView to Rust
    I want both transports to land identically on the SessionManagerHandle for /blocklist RPCs
    So that the WebSocket and embedded paths cannot diverge as the feature grows

  Scenario: Embedded and WebSocket blocklist_list both reach the stub
    Given a StubSessionManagerHandle seeded with three rules behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When blocklist_list is called via the embedded transport
    And blocklist_list is called via the WebSocket transport
    Then the stub's blocklist_list_calls counter equals 2
    And both calls return a Vec of length 3
    And each entry has identical id, pattern, action, source fields across the two transports
