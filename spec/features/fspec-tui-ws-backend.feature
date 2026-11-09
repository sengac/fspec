@BUG-582
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
  bind_and_serve rpc-server bound to 127.0.0.1:0. The source-shape scenario
  that pinned tokio_tungstenite::connect_async was retired by BUG-667: every
  connect goes through codelet_rpc_server::ws_connect (RPC-458's trust),
  covered by a behaviour test.

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
