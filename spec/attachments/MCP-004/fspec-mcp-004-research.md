# Research: MCP HTTP custom headers are ignored

## Observed bug

`rust/tools/src/mcp.rs:684` defines `connect_http` and accepts an optional headers map. At `:694` it constructs `StreamableHttpClientTransport::from_uri(url)`, and at `:697` explicitly warns that custom headers are unsupported and continues without them. The existing feature expectations in `spec/work-units.json` for MCP-001 say headers allow tokens and credentials, so this behavior contradicts the exposed interface.

The Paseo Bots relay mounts per-chat tools as HTTP MCP with `Authorization: Bearer ...` (`/tmp/paseo-bots/server/relay.ts:131`); connected apps use an equivalent header at `:123`. Dropping the header means these servers cannot authenticate the client. This is not merely a Paseo setting. Snapshots: fspec `344d00b4`, fspec-paseo `f64171f`, `/tmp/paseo-bots` `39a7000`; verify current versions before work.

## Desired behavior and scenarios

- Forward configured valid headers across the full Streamable HTTP MCP session, including initialization, discovery, calls and any applicable stream/reconnect requests. An authenticated loopback server verifies the actual received header for a real handshake and tool call; a request without required credentials fails.
- No configured headers must remain compatible with an ordinary unauthenticated server. Invalid header names or values must return a sanitized error instead of silently dropping the header or connecting with different authentication semantics.
- Never include header values, tokens, or credential-bearing URLs in logs, errors, debug output, transcripts or persisted configuration. Verify negative cases including failed handshakes and server responses.
- Test overlapping sessions with different tokens to prevent cross-session header leakage, plus cleanup and failure paths. Avoid global mutable headers.

## Implementation investigation

`connect_http` uses rmcp's Streamable HTTP transport. Investigate the pinned rmcp and reqwest versions and transport custom-client construction before choosing a compatible client; the source TODO mentions dependency-version alignment. Preserve timeouts, MCP notifications, session ownership and current no-header behavior. Update existing feature scenarios and tests first; test both direct ConnectMCP and the provider-facing API once MCP-003 exists.

## Boundaries

This card supplies HTTP header support, not fspec-paseo config forwarding or a new RPC. MCP-003 is the separate per-session configuration endpoint. The `/tmp` bot checkout is historical, not proof of latest upstream behavior. Use synthetic test tokens only and scoped crate/target test commands.
