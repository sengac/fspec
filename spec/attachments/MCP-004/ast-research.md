## fspec astgrep --pattern 'connect_http($$$A)' --lang rust --path rust/tools/src
rust/tools/src/mcp/connect_tool.rs:140:25:connect_http(
rust/tools/src/mcp/host_config.rs:133:17:connect_http(


## fspec astgrep --pattern 'connect_http($$$A)' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'StreamableHttpClientTransport::from_uri($A)' --lang rust --path rust/tools/src
rust/tools/src/mcp/http_and_session_registry.rs:67:21:StreamableHttpClientTransport::from_uri(url)


## fspec astgrep --pattern 'HostMcpTransport::Http { $$$F }' --lang rust --path rust/tools/src
No matches found

## fspec astgrep --pattern 'SessionMcpTransport::Http { $$$F }' --lang rust --path rust/sessions/src
No matches found

## fspec astgrep --pattern 'SessionMcpTransport::Http { $$$F }' --lang rust --path rust/rpc-server/tests
No matches found

## grep (struct-pattern matches ast-grep does not resolve)

- rust/sessions/src/session_mcp.rs:18 `SessionMcpTransport::Http { url } => HostMcpTransport::Http { url }` — the wire → host mapping; must carry headers.
- rust/tools/src/mcp/host_config.rs:132 `HostMcpTransport::Http { url } => connect_http(.., None, ..)` — the daemon path passes no headers today.
- rust/tools/src/mcp/host_config.rs:192 `redact` — HTTP secrets are only the url; header values must join them.
- rust/rpc-server/tests/mcp003/lifecycle.rs:98 `SessionMcpTransport::Http { url }` — MCP-003 test literal; gains `headers: BTreeMap::new()`.

## rmcp 1.2.0 (registry source)

- `StreamableHttpClientTransport::<reqwest::Client>::from_config(StreamableHttpClientTransportConfig)` (common/reqwest/streamable_http_client.rs:278).
- `StreamableHttpClientTransportConfig::with_uri(..).custom_headers(HashMap<HeaderName, HeaderValue>)` — applied to every POST, GET stream and DELETE by the worker.
- `apply_custom_headers` rejects reserved `accept`, `mcp-session-id`, `last-event-id` with `StreamableHttpError::ReservedHeaderConflict(name)`; `mcp-protocol-version` passes through.
- rmcp uses reqwest 0.13, codelet-tools reqwest 0.12; both build on the single `http 1.4` in Cargo.lock, so `reqwest::header::{HeaderName, HeaderValue}` are the same types rmcp takes — no dependency alignment needed.
