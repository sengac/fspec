# AST research: REFAC-035
- `rust/tools/src/mcp/host_config.rs`: lines 225-294 hold `async fn establish(server, injection_tx, connections)` and `fn redact(message, transport)`. They are used only by `configure_host_mcp_servers`.
- `rust/tools/src/mcp.rs` declares `pub mod host_config;`. A fragment pulled in with `include!` from `host_config.rs` resolves against `rust/tools/src/mcp/`.
- Coverage files that map `host_config.rs`: declarative-session-mcp-server-sets, mcp-http-transport-headers, mcp-legacy-sse-transport, mcp-stdio-structured-arguments, session-mcp-server-lifecycle and session-mcp-server-configuration (`.feature.coverage`).
