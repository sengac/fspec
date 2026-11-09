# AST research: BUG-667

## fspec astgrep --pattern 'tokio_tungstenite::connect_async($$$)' --lang rust --path rust/fspec-tui/src
```
rust/fspec-tui/src/transport/websocket/construction.rs:134:36:tokio_tungstenite::connect_async(url.as_str())

```

## fspec astgrep --pattern 'connect_async' in tests pinning it
```
rust/fspec-tui/tests/ws_backend_smoke.rs:115:    // @step Then it calls `tokio_tungstenite::connect_async(url)` directly
rust/fspec-tui/tests/ws_backend_smoke.rs:117:        src.contains("tokio_tungstenite::connect_async("),
```

## Findings

- construction.rs `connect_with_supervisor` is the only fspec-tui connect that bypasses `codelet_rpc_server::ws_connect` (RPC-458 trust); its reconnects (`run_supervisor`) already use `ws_connect_trusting`.
- fspec-tui/tests/ws_backend_smoke.rs `websocket_backend_connect_uses_tokio_tungstenite_connect_async_directly` (RPC-008, fspec-tui-ws-backend.feature scenario "WebSocketFspecBackend.connect uses tokio_tungstenite::connect_async directly") pins the substring `tokio_tungstenite::connect_async(` in the composed websocket.rs source; it is retired.
- Callers of connect_with_supervisor are tests only (auto_reconnect_slice2_rpc011, disconnect_dialog_slice1_rpc011); they use plain ws:// daemons, which ws_connect serves unchanged.
