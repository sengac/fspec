@done
@tui
@RPC-059
@rpc
@rust
@parity
@loop-management
Feature: /loop cross-transport parity
  """
  Both EmbeddedFspecBackend (in-process embedded transport) and
  WebSocketFspecBackend (tarpc over WebSocket) must land identically on
  the same StubSessionManagerHandle for every new RPC method introduced
  by RPC-059:

    * loop_add
    * loop_cancel
    * loop_list

  Mirrors the RPC-058 cross-transport parity test — each transport
  invocation increments the same per-stub counter and returns the same
  payload.
  """

  Background: User Story
    As a developer porting the AgentView to Rust
    I want both transports to land identically on the SessionManagerHandle for the /loop RPCs
    So that the WebSocket and embedded paths cannot diverge as the feature grows

  Scenario: Embedded and WebSocket loop_add both reach the stub
    Given a StubSessionManagerHandle seeded with a RegisteredLoop { id: "a1b2c3d4", session_id: SessionId::new("s-1"), prompt: "check the build", interval_seconds: 30, created_at: "2026-05-24T00:00:00Z", expires_at: "2026-05-27T00:00:00Z", last_run_at: None } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When loop_add is called via the embedded transport with session_id "s-1" and interval_seconds 30 and prompt "check the build"
    And loop_add is called via the WebSocket transport with session_id "s-1" and interval_seconds 30 and prompt "check the build"
    Then the stub's loop_add_calls counter equals 2
    And both calls return Ok(RegisteredLoop) with byte-identical field values

  Scenario: Embedded and WebSocket loop_cancel both reach the stub
    Given a StubSessionManagerHandle seeded to return Ok(true) for loop_cancel behind both transports
    When loop_cancel is called via the embedded transport with id "a1b2c3d4"
    And loop_cancel is called via the WebSocket transport with id "a1b2c3d4"
    Then the stub's loop_cancel_calls counter equals 2
    And both calls return Ok(true)

  Scenario: Embedded and WebSocket loop_list both reach the stub
    Given a StubSessionManagerHandle seeded with two RegisteredLoop rows for session "s-1" behind both transports
    When loop_list is called via the embedded transport for session_id "s-1"
    And loop_list is called via the WebSocket transport for session_id "s-1"
    Then the stub's loop_list_calls counter equals 2
    And both calls return a Vec of length 2
    And each entry has identical id, session_id, prompt, interval_seconds, created_at, expires_at, last_run_at fields across the two transports
