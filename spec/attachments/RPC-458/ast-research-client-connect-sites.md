# AST research: RPC-458 client connect sites

## fspec astgrep --pattern 'codelet_rpc_server::ws_connect($$$)' --lang rust --path rust
```
rust/fspec/tests/bug619_wss_connect_with_two_crypto_providers.rs:83:9:codelet_rpc_server::ws_connect(&url, None)
rust/fspec-tui/src/transport/websocket/construction.rs:17:30:codelet_rpc_server::ws_connect(url.as_str(), bearer.as_deref())
rust/fspec-tui/src/transport/websocket/construction.rs:76:31:codelet_rpc_server::ws_connect(url.as_str(), bearer.as_deref())
rust/fspec-tui/src/transport/websocket/reconnect.rs:89:30:codelet_rpc_server::ws_connect(url.as_str(), bearer.as_deref())

```

## fspec astgrep --pattern 'tokio_tungstenite::connect_async($$$)' --lang rust --path rust
```
rust/rpc-server/src/client/connect.rs:24:8:tokio_tungstenite::connect_async(request)
rust/rpc-server/tests/bug241_provider_waiting_wire.rs:29:23:tokio_tungstenite::connect_async(format!("ws://{address}"))
rust/rpc-server/tests/bug220_interruption_wire.rs:53:23:tokio_tungstenite::connect_async(format!("ws://{address}"))
rust/rpc-server/tests/sess003_focus_transport.rs:98:25:tokio_tungstenite::connect_async(format!("ws://{address}"))
rust/rpc-server/tests/sess003_focus_transport.rs:99:25:tokio_tungstenite::connect_async(format!("ws://{address}"))
rust/rpc-server/tests/sess003_focus_transport.rs:175:23:tokio_tungstenite::connect_async(format!("ws://{address}"))
rust/rpc-server/tests/rpc447_bearer_auth.rs:39:18:tokio_tungstenite::connect_async(url.as_str())
rust/sessions/tests/bug251/transport.rs:30:27:tokio_tungstenite::connect_async(format!("ws://{address}/"))
rust/fspec/tests/daemon_lifecycle_rpc011.rs:48:25:tokio_tungstenite::connect_async(&url)
rust/fspec/tests/daemon_lifecycle_rpc011.rs:51:25:tokio_tungstenite::connect_async(&url)
rust/fspec/tests/daemon_lifecycle_rpc011.rs:157:13:tokio_tungstenite::connect_async(&url)
rust/fspec/tests/daemon_lifecycle_rpc011.rs:251:19:tokio_tungstenite::connect_async(&url)
rust/fspec/tests/client_mcp.rs:382:24:tokio_tungstenite::connect_async(url)
rust/fspec/tests/status_subcommand_rpc011.rs:82:22:tokio_tungstenite::connect_async(&url)
rust/fspec/tests/status_subcommand_rpc011.rs:85:22:tokio_tungstenite::connect_async(&url)
rust/fspec-tui/src/transport/websocket/construction.rs:109:36:tokio_tungstenite::connect_async(url.as_str())

```

## fspec astgrep --pattern 'SavedDaemon { $$$ }' --lang rust --path rust
```
rust/common/src/saved_daemons.rs:324:18:SavedDaemon {
rust/common/src/saved_daemons.rs:389:22:SavedDaemon {
rust/common/tests/tui123_saved_daemon_rules.rs:12:5:SavedDaemon {
rust/common/tests/rpc447_saved_daemons.rs:10:5:SavedDaemon {
rust/fspec/src/daemons_cmd.rs:32:17:SavedDaemon {
rust/fspec-tui/tests/tui121_manage_saved_daemons_from_the_tui.rs:209:9:SavedDaemon {
rust/fspec-tui/tests/tui121_manage_saved_daemons_from_the_tui.rs:247:9:SavedDaemon {

```

## fspec astgrep --pattern 'connect_daemon($$$)' --lang rust --path rust
```
rust/fspec/src/daemon_directory.rs:105:23:connect_daemon(&url, token, DAEMON_CONNECT_TIMEOUT)
rust/fspec-tui/tests/tui123_daemon_connect_failures.rs:47:11:connect_daemon(&url, Some(TOKEN.to_string()), timeout)

```

## fspec astgrep --pattern 'resolve_connect($$$)' --lang rust --path rust
```
rust/fspec/src/daemons_cmd/tests.rs:15:25:resolve_connect("wss://daemon.example:8443", Some("s3cret".to_string()))
rust/fspec/src/daemons_cmd/tests.rs:29:20:resolve_connect("ws://127.0.0.1:9000", None)

```

## Findings

- Every TUI/CLI connect funnels through `codelet_rpc_server::ws_connect` except `WebSocketFspecBackend::connect_with_supervisor`, which calls `tokio_tungstenite::connect_async` directly (it must use ws_connect too).
- `ws_connect` uses tokio-tungstenite's default rustls connector (webpki roots only). It becomes `connect_async_tls_with_config` with a `Connector::Rustls` built from webpki roots + FSPEC_DAEMON_CA + the saved daemon's CA, ring provider (BUG-619).
- Saved-daemon credentials are resolved in two places: `fspec/src/daemons_cmd.rs::resolve_connect` (fspec status/client --connect <name>) and `fspec/src/daemon_directory.rs::ClientDaemonDirectory::connect` (switcher). Both must carry the saved CA; the TUI backend keeps it for the reconnect supervisor beside the bearer.
- `SavedDaemon { name, url, token }` struct literals: saved_daemons.rs (save/update), daemons_cmd.rs, and four test files (rpc447_saved_daemons, tui123_saved_daemon_rules, tui121_manage_saved_daemons_from_the_tui, tui123_support) that need `ca: None`.
- The other `connect_async` hits are tests dialing plain ws:// test listeners; the bridge relay (tools/src/bridge_relay.rs) dials the bridge service, not a daemon.
