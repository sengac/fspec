# MCP-003 design: configure a session's MCP servers through the daemon

## Problem (evidence)

- Per-session MCP state exists (`rust/tools/src/mcp/http_and_session_registry.rs` `init_mcp_session`
  / `cleanup_mcp_session` / `get_mcp_connections`, keyed by session UUID; initialised at session
  creation, `rust/sessions/src/session_creation_helper.rs`), and the agent gathers a session's MCP
  tool wrappers when it builds the agent (`rust/agent-loop/src/dispatch.rs`
  `gather_mcp_tool_wrappers`). The only way to connect a server is the model-facing `ConnectMCP`
  tool (`rust/tools/src/mcp/connect_tool.rs`); the session-state accessor is private.
- No FspecService method lets a daemon client configure servers
  (`rust/rpc/src/service_declaration/*`), so fspec-paseo cannot forward `input.config.mcpServers`.
- The daemon's only authentication is RPC-447's handshake bearer
  (`rust/rpc-server/src/server.rs:143-160`, `SharedFspecService::authorizes`); the service
  implementation (`FspecServiceImpl`, `rust/rpc/src/lib.rs:83`) does not know whether its
  connection was authenticated.

## User decision (architecture note on the card)

Configuring MCP servers can launch local processes and reach network addresses, so it requires
daemon authentication: refused unless the daemon was started with `FSPEC_DAEMON_TOKEN` and the
request carried `Authorization: Bearer <token>`. On a daemon without a token the method is
disabled with an explicit error naming `FSPEC_DAEMON_TOKEN`. No loopback trust.

## Chosen design

### Authentication

- `FspecServiceImpl` gains `bearer_authenticated: bool`. `FspecServiceImpl::new` (in-process:
  rpc-embedded, tests) sets `false`. New `FspecServiceImpl::for_authenticated_connection(inner)`
  sets it to `inner.requires_bearer()` — used by rpc-server for both WebSocket paths after the
  RPC-447 handshake check passed, so on a token-armed daemon every such connection presented the
  bearer, and on a token-less daemon it is `false`.
- `configure_session_mcp_servers` refuses, in order: no token configured →
  "MCP server configuration is disabled: start the daemon with FSPEC_DAEMON_TOKEN and connect with
  `Authorization: Bearer <token>`"; token configured but connection not bearer-authenticated →
  "MCP server configuration requires a connection authenticated with the daemon's
  FSPEC_DAEMON_TOKEN bearer".

### Wire (new types in codelet-rpc-types `session_mcp.rs`; bincode field order as listed)

```rust
pub struct SessionMcpServer {        // fields: name, transport, timeout_secs
    pub name: String,
    pub transport: SessionMcpTransport,
    pub timeout_secs: u64,
}
pub enum SessionMcpTransport {       // variant 0: Stdio { command, env }; variant 1: Http { url }
    Stdio { command: String, env: BTreeMap<String, String> },
    Http { url: String },
}
pub struct SessionMcpServerTools {   // fields: name, tools (qualified mcp__<server>__<tool>)
    pub name: String,
    pub tools: Vec<String>,
}
```

New method appended LAST to `FspecService` (new declaration fragment
`service_declaration/session_mcp.rs`, chained after supervision_credentials; RPC-449 frozen order
gains it as the final line):

```rust
async fn configure_session_mcp_servers(
    session_id: SessionId,
    servers: Vec<SessionMcpServer>,
) -> Result<Vec<SessionMcpServerTools>, String>;
```

`Debug` for `SessionMcpServer`/`SessionMcpTransport` is hand-written and prints neither the
command, the url's query nor any environment value.

### Layers

- rpc: declaration + implementation fragments, `mcp_dispatch` catalog/dispatch/validate entry
  (group_42; destructive, not read-only).
- core `SessionManagerHandle::configure_session_mcp_servers(&self, &SessionId, Vec<..>) ->
  Pin<Box<dyn Future<Output = Result<Vec<..>, String>> + Send>>` with a default that refuses
  (the stub and other handles report that MCP configuration is unavailable on them).
- sessions: the SessionManager implementation resolves the live session (unknown → error naming
  it), refuses a busy session, maps the wire types and calls the tools-level function.
- tools: new child module `mcp/host_config.rs` (`pub mod host_config`), sharing the ConnectMCP
  path: it validates the request, takes the session's injection channel and connection map from
  the same registry (`get_mcp_session_state`) and calls the same `connect_stdio` / `connect_http`.
- fspec-tui `FspecBackend::configure_session_mcp_servers` (required, BUG-331 style): websocket
  calls the RPC; embedded refuses (in-process is never bearer-authenticated); test mocks refuse.

### Behaviour

- Validation before any connection: at least one server; names non-empty, `[A-Za-z0-9_-]`, no
  `__` (it would break `mcp__<server>__<tool>` parsing); unique in the request; not already
  connected in the session; timeout > 0; a stdio command must not be blank; an http url must
  parse as `http(s)://`.
- Atomic: servers connect in request order; on the first failure every server connected by this
  request is disconnected (child processes cancelled), and the error names the failing server and
  its connect message (which never contains the command or environment).
- Ownership: the connection map is the session's; tools appear to that session's agent at its next
  agent build; `cleanup_mcp_session` (session destroy) cancels them.
- Policy: host configuration is authorised by the daemon bearer, not by the agent's pre-tool-use
  hooks (those gate model tool calls); the configured tools themselves still run through the normal
  tool path when the model calls them.
- Not persisted: a resumed session must be configured again by its client (secrets are never
  written to manifests or transcripts).

## Alternatives rejected

- Loopback trust or a separate unauthenticated local endpoint: refused by the user decision.
- Reusing ConnectMCP by injecting a synthetic tool call: couples host configuration to the model's
  tool loop, its hooks and transcript.
- Partial success: a client could not tell which servers it owns; atomic is explicit.
- Accepting `sse` as Streamable HTTP: wrong protocol (MCP-006 implements legacy SSE).

## Wire / API changes (for bincode clients such as fspec-paseo)

- New `FspecService` method `configure_session_mcp_servers(session_id: SessionId, servers:
  Vec<SessionMcpServer>) -> Result<Vec<SessionMcpServerTools>, String>`, appended last (variant
  index = previous count).
- New types as above (field and variant order as listed).
