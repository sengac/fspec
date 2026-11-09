# MCP-004 root cause

## Symptom
An HTTP MCP server that requires credentials (for example `Authorization: Bearer …`, as the Paseo Bots relay does)
refuses fspec: the handshake fails with 401 although the ConnectMCP call supplied the header.

## Cause
`connect_http` (rust/tools/src/mcp/http_and_session_registry.rs) built the transport with
`StreamableHttpClientTransport::from_uri(url)` and, when headers were given, logged a warning and connected
without them. The TODO blamed a reqwest version mismatch, but rmcp 1.2's
`StreamableHttpClientTransportConfig` already takes `custom_headers` (typed on the shared `http 1.4` crate)
and applies them to every request of the session. The daemon path (MCP-003 `configure_session_mcp_servers`)
had no way to express headers at all: `SessionMcpTransport::Http` carried only `url`.

## Fix
- `connect_http` parses every configured header into `HeaderName`/`HeaderValue` (marked sensitive) and builds
  the transport with `from_config(with_uri(url).custom_headers(..))`. An invalid name or value fails the connect
  with an error naming the header, never its value. Reserved headers are refused by rmcp
  (`ReservedHeaderConflict`), surfaced as a failed handshake.
- Wire type `SessionMcpTransport::Http` gains `headers: BTreeMap<String, String>` (bincode order `url`, `headers`);
  its Debug prints header names only. `HostMcpTransport::Http` carries them to `connect_http`, and `redact`
  removes header values from errors.
- Headers live in each connection's own transport config: no global state, so concurrent sessions with different
  tokens never see each other's headers.
