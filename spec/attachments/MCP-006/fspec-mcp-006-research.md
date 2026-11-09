# Research: optional legacy SSE MCP transport

## Why this card exists

Paseo's provider contract allows MCP definitions with `type: "http" | "sse"`, URL and optional headers (`/Users/rquast/projects/fspec-paseo/docs/paseo-provider-contract.md:794`). The existing fspec `ConnectMCP` transport path distinguishes stdio and Streamable HTTP (`rust/tools/src/mcp.rs:1285`); do not assume that an SSE endpoint is interchangeable with Streamable HTTP. Paseo Bots itself supplies its built-in relay as `type: "http"` (`/tmp/paseo-bots/server/relay.ts:131`), so legacy SSE is **not required** for that core bot-tools path. User-configured servers may use it.

Snapshots: fspec `344d00b4`, fspec-paseo `f64171f`, `/tmp/paseo-bots` `39a7000`; validate current versions and actual supported SDK transports before choosing an implementation.

## Discovery decision

Confirm whether compatibility with user-configured legacy SSE servers is wanted now, and whether the pinned rmcp client offers a supported SSE transport. If it is out of scope, ensure the session configuration endpoint returns an explicit unsupported-transport error instead of silently ignoring or treating SSE as Streamable HTTP. Do not claim full Paseo MCP transport compatibility without it.

## If supported

- Preserve SSE transport selection and forward authentication headers without leaking credentials. Coordinate with MCP-004; authenticated SSE must not silently downgrade to anonymous.
- Use a real local legacy SSE MCP server for initialization, discovery, tool invocation, server events, disconnect and cleanup. Test timeout, 401, inaccessible endpoint, session isolation and resumed-session reconfiguration.
- Review security of redirect handling and credential forwarding, lifecycle/cancellation, and whether SSE is compatible with current rmcp versions and the supported MCP protocol. Avoid introducing global shared client state.

## Scope boundaries

MCP-003 tracks a daemon-facing session configuration API, MCP-004 Streamable HTTP headers, and MCP-005 structured stdio. fspec-paseo must separately forward its server definitions; this card is not needed to send the built-in Bots relay's Streamable HTTP requests.
