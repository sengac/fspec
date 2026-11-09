# MCP-006 AST research

## fspec astgrep --pattern 'connect_http($$$A)' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/connect_tool.rs:144:25:connect_http(
rust/tools/src/mcp/host_config.rs:152:17:connect_http(


## fspec astgrep --pattern 'McpTransport::Http' --lang rust --path rust/tools/src/mcp
rust/tools/src/mcp/helper_tests_01.rs:231:44:McpTransport::Http
rust/tools/src/mcp/connect_tool.rs:139:21:McpTransport::Http
rust/tools/src/mcp/tests_02.rs:14:70:McpTransport::Http
rust/tools/src/mcp/tests_02.rs:178:62:McpTransport::Http
rust/tools/src/mcp/tests_03.rs:53:25:McpTransport::Http
rust/tools/src/mcp/tests_03.rs:174:52:McpTransport::Http
rust/tools/src/mcp/http_and_session_registry.rs:197:20:McpTransport::Http
rust/tools/src/mcp/tests_01.rs:246:25:McpTransport::Http
rust/tools/src/mcp/tests_01.rs:261:66:McpTransport::Http
rust/tools/src/mcp/helper_tests_02.rs:201:25:McpTransport::Http


## fspec astgrep --pattern 'HostMcpTransport::Http { $$$F }' --lang rust --path rust/tools/src/mcp
No matches found

## fspec astgrep --pattern 'impl Transport<$R> for $T { $$$B }' --lang rust --path rust/tools/src
No matches found

## rmcp 1.2.0 (registry source)

- `rmcp::transport::Transport<R>`: `send(&mut self, TxJsonRpcMessage<R>) -> impl Future + Send + 'static`,
  `receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<R>>>`, `close(&mut self)`; any `Transport`
  is `IntoTransport` (transport.rs:155), so `handler.serve(transport)` accepts a custom client transport.
- rmcp 1.2 ships Streamable HTTP, child process, async read/write, sink/stream and websocket transports — no legacy
  SSE client (`transport/` has no `sse_client.rs`).
- `eventsource-stream 0.2.3` is already in Cargo.lock (rig-core patch, codelet-providers): reusable SSE parser over
  `reqwest::Response::bytes_stream()` (codelet-tools has reqwest 0.12 with `stream`).
