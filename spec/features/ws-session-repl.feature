@done
@streaming
@rust
@tarpc
@session-management
@rpc
@websocket
@RPC-007
@BUG-520
Feature: WebSocket session REPL push channel
  """
  codelet-rpc-server gains five new session RPCs (list_sessions, create_session,
  send_input, interrupt, get_session_status) reachable from a WebSocket client
  via tarpc-generated FspecServiceClient over bincode-encoded
  Envelope::Rpc(Vec<u8>) frames. FspecWsClient gains chunks_rx() and
  logs_rx() returning broadcast::Receivers populated by the server-side
  fan-out tasks. The cross-transport parity invariant: byte-equal chunk
  sequences on the embedded path and the WS path for the same StubProvider
  input.
  BUG-520: the stub ends a turn about 5 ms after send_input, so polling the
  status during or after the turn raced it. The status scenario holds the
  stub's turn open while it polls for Running, then releases it and waits for
  the session's Idle status change on the WebSocket push channel before
  polling again.
  """

  Background: User Story
    As a developer building remote tooling against a running fspec workspace
    I want session lifecycle and streaming over the WebSocket transport with the same shape as the embedded path
    So that a remote ratatui REPL or a future CLI can observe streaming output without depending on NAPI or the in-process embedded transport

  @rpc
  @websocket
  @session
  Scenario: WebSocket list_sessions matches the embedded transport result
    Given a developer has started codelet-rpc-server bound to 127.0.0.1 with a SessionManagerHandle backed by the same SessionManager
    And a WebSocket client is connected to the server
    When the client calls FspecServiceClient::list_sessions(context::current()) over the WebSocket transport
    Then the call returns Ok(Vec<SessionInfo>) equal to the result of the embedded list_sessions in the parity scenario
    And the SessionInfo entries serialize/deserialize via bincode-of-Envelope without shape mismatch

  @rpc
  @websocket
  @streaming
  @parity
  Scenario: WebSocket create_session + send_input yields the same chunk sequence as embedded (cross-transport parity)
    Given a WebSocket client connected to codelet-rpc-server with the StubProvider feature enabled
    And the client has subscribed to FspecWsClient::chunks_rx() before sending input
    When the client calls create_session(None) and send_input(session_id, "hi")
    Then send_input returns Ok(()) immediately
    And the chunks observed on chunks_rx() are byte-equal to the chunks observed on the embedded path for the same input
    And every chunk arrived as a bincode-encoded Envelope::Event { session_id, chunk } frame on the WebSocket wire

  Scenario: WebSocket get_session_status reflects Idle to Running to Idle transitions
    Given a session is created via create_session on the WebSocket transport with the StubProvider
    And the stub provider holds each turn open until the caller releases it
    When the caller calls get_session_status(session_id) before any send_input
    Then the returned SessionStatus is Idle
    When the caller calls send_input(session_id, "hi") and calls get_session_status(session_id) while the turn is held
    Then the returned SessionStatus is Running
    When the caller releases the turn, receives the session's Idle status change on the WebSocket client and calls get_session_status(session_id) again
    Then the returned SessionStatus is Idle
