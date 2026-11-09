# Research: session-scoped MCP configuration through the daemon

## Goal and scope

Let a trusted, authenticated daemon client configure named MCP servers for one fspec session before the first agent turn, so external clients such as fspec-paseo can supply their tools. This is an fspec daemon/API card, not the fspec-paseo forwarding implementation. Research snapshot: fspec `344d00b4`, fspec-paseo `f64171f`, and the existing `/tmp/paseo-bots` checkout `39a7000`; recheck the current code before implementation.

## Evidence

- `rust/sessions/src/session_creation_helper.rs:285` initializes per-session MCP state; `rust/tools/src/mcp.rs:855` maintains injection channels and a connection map keyed by session UUID.
- `rust/tools/src/mcp.rs:1266` handles the agent-facing `ConnectMCP` tool, obtains per-session state, and invokes the transport-specific connection functions. `rust/agent-loop/src/dispatch.rs:58` gathers session MCP wrappers into the agent. The state accessor is currently private (`rust/tools/src/mcp.rs:910`). Reuse the underlying implementation rather than asking the model to call a tool.
- `rust/rpc/src/mcp_dispatch/dispatch.rs` exposes the daemon's RPC methods; no provider-facing per-session MCP configuration method was found in the inspected RPC surface. Inspect the tarpc service definition, transport, schema generation, compatibility conventions and authentication before designing the endpoint.
- `/Users/rquast/projects/fspec-paseo/server/session.ts:897` opens a session but does not consume `input.config.mcpServers`; the Paseo contract describes stdio, HTTP and SSE definitions in `/Users/rquast/projects/fspec-paseo/docs/paseo-provider-contract.md:794`.
- Paseo Bots provides named `bots`, optional connected-apps and user-configured servers in `/tmp/paseo-bots/shared/bot.ts:584`; its own relay uses per-chat bearer-authenticated HTTP (`/tmp/paseo-bots/server/relay.ts:131`).
- Workspace routing is already multi-workspace: `server/session.ts:863` and `server/workspaces.ts:28` in fspec-paseo, and `rust/sessions/src/default_project.rs:28` in fspec. It is not necessary to regress to a single workspace to solve this.

## Proposed behavior and examples

1. A client creates session A, configures a named server and receives success only when the contractually required connection/handshake is ready; a first prompt can see and invoke its tools. Session B must not see A's tools, credentials or connection state.
2. Unknown/destroyed session, invalid definitions, unsupported transports, handshake failure, timeout and authentication failure produce actionable, sanitized errors; no misleading success or leaked child processes. Define atomic versus partial behavior explicitly for multi-server setup.
3. Close/destroy must release session-owned servers; distinguish closing a provider connection from destroying a daemon session. A resumed session needs fresh configuration from the client where applicable; never persist bearer tokens or environment secrets into manifests/transcripts/logs by default.
4. Define duplicate names, repeated configuration, replacement/disconnection, request cancellation and concurrent open/configure/destroy behavior. Ensure the setup barrier prevents an early first turn racing tool registration.
5. Exercise real daemon RPC and local test MCP servers, including two isolated sessions, error/cleanup paths, and resume. Preserve existing `ConnectMCP` and mid-turn tool behavior.

## Security and design decisions to resolve

- Daemon authentication and authorization are essential because stdio config can launch arbitrary local processes and HTTP config can reach network addresses. Confirm the existing trust boundary and avoid introducing an unauthenticated tool-execution endpoint; do not assume authorization is already sufficient.
- Decide server-config scope, exact API shape, which transports the initial version supports, required vs optional server failure, ordering, replay and secret handling. Treat upstream process/env values as sensitive; prevent secrets in Debug, errors and recorded events.
- The per-session connection APIs should share a common internal implementation with the agent tool and retain hooks/policy deliberately. Decide if host-provided configuration is subject to the same approval policy, rather than bypassing it accidentally.
- Existing work unit RPC-062 explicitly avoided exposing internal MCP plumbing; expose only a deliberate, bounded configuration contract rather than internal channels/maps.
- MCP-004 handles HTTP headers, MCP-005 structured stdio arguments, MCP-006 legacy SSE. These are related capabilities, not prerequisites for designing the RPC; do not block independently useful stdio/basic HTTP API work unnecessarily. fspec-paseo must separately forward configuration and apply its persona.

## Validation

Write acceptance scenarios before code and use real loopback MCP test servers and real daemon sessions. Verify tool discovery and invocation before first prompt, concurrent isolation, failure/redaction, cleanup and reconnection; do not rely only on source-shape tests. Run scoped crate/target tests per repository guidance.
