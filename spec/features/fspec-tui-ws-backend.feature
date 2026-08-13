@done
@parity
@websocket
@tarpc
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: WebSocketFspecBackend smoke + connect-shape
  End-to-end smoke test exercising WebSocketFspecBackend against a real
  bind_and_serve rpc-server bound to 127.0.0.1:0 plus the source-shape
  invariant that connect uses tokio_tungstenite::connect_async directly
  (no helper in codelet-rpc-server, no envelope/bincode/framing code in
  fspec-tui).

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want WebSocketFspecBackend.connect to open the WS via tokio_tungstenite::connect_async, hand the resulting stream to ws_client_connect, and round-trip list_work_units across the wire
    So that the WS transport from RPC-005/006/007 has its first concrete trait-side consumer with no envelope code leaking into the TUI crate

  Scenario: WebSocketFspecBackend smoke test round-trips list_work_units across the WS wire
    Given a `bind_and_serve` rpc-server running on 127.0.0.1:0 with a tempdir-backed WorkUnitsWatcher
    And a WebSocketFspecBackend constructed via `WebSocketFspecBackend::connect(ws_url).await?`
    When the test calls `backend.list_work_units().await`
    Then the returned Vec<WorkUnitInfo> equals what an EmbeddedFspecBackend wrapping the same service would have returned
    When the test subscribes via `backend.work_units_rx()`
    Then the initial WorkUnitsUpdate snapshot frame from RPC-006 is observed within 5 seconds

  Scenario: WebSocketFspecBackend.connect uses tokio_tungstenite::connect_async directly
    Given rust/fspec-tui/src/transport/websocket.rs exists
    When I inspect the body of `WebSocketFspecBackend::connect`
    Then it calls `tokio_tungstenite::connect_async(url)` directly
    And it hands the resulting WebSocketStream to `codelet_rpc_server::ws_client_connect()`
    And it stores the resulting FspecWsClient on the struct
    And no envelope, bincode, or framing code lives in rust/fspec-tui/src/transport/
