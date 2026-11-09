# MCP-006 design: legacy SSE MCP client transport

User decision (2026-11-01): implement a legacy SSE MCP client in fspec (rmcp 1.2 has none); SSE is never treated as
Streamable HTTP.

## Protocol (MCP 2024-11-05 "HTTP with SSE")
1. `GET <url>` with `Accept: text/event-stream` and the configured headers.
2. The server's first event is `event: endpoint`, `data: <uri>` — the URI (usually relative, with a session query) to
   which the client POSTs every JSON-RPC message.
3. Responses, requests and notifications from the server arrive as `event: message` events whose data is one JSON-RPC
   message. POSTs are answered with 202 (or 200).

## Implementation
- `rust/tools/src/mcp/sse_transport.rs` (`pub mod sse_transport`): `LegacySseTransport::connect(url, headers)` opens
  the stream (reqwest 0.12, `eventsource-stream`), maps 401 to an authentication error and other non-success statuses
  to a status error, waits for the `endpoint` event, resolves it against the SSE url and refuses another origin
  (scheme, host and port must match) so credentials are never forwarded. A reader task forwards each `message` event
  (parsed as `ServerJsonRpcMessage`) through an mpsc channel; when the stream ends the channel closes and rmcp ends
  the service. `Transport::send` POSTs the message with the same headers and fails on a non-success status.
  `close` and `Drop` abort the reader task. Headers are parsed by MCP-004's `parse_mcp_http_headers` (values
  sensitive). No global state: each connection owns its client, headers and reader.
- `connect_sse` (ConnectMCP transport `"sse"`) and `connect_http` share a `remote_connect` fragment: the
  serve-with-timeout error mapping and the discovery/caching tail. The whole SSE connect (stream open, endpoint wait,
  handshake) is bounded by the configured timeout.
- Wire: `SessionMcpTransport::Sse { url, headers }` (variant 2; Debug prints header names only). `HostMcpTransport::Sse`
  validates url and headers like Http and redacts url and header values from errors.

## Tests
`rust/rpc-server/tests/mcp006_legacy_sse_transport.rs` against the fixture binary's new legacy SSE modes:
`--sse [--bearer T]`, `--sse-silent` (never sends the endpoint), `--sse-foreign` (endpoint on another origin).
