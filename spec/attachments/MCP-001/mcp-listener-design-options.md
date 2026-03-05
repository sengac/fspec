# Dynamic MCP: Tool-Driven MCP Integration via ConnectMCP

## What is Dynamic MCP?

**Dynamic MCP** is an architecture where MCP server connections are established through tool calls at runtime, rather than through static configuration files at session startup. The agent decides when to connect, receives structured feedback on success or failure, and gains typed MCP tools dynamically mid-session.

This stands in contrast to how every existing agent (Codex, OpenCode) implements MCP: **Static MCP**, where servers are pre-configured, connected before the agent is involved, and failures are invisible to the LLM.

The key insight: **MCP connection management is agent work, not infrastructure work.** The agent should see failures, reason about alternatives, and connect to servers when the context demands it — especially when skills describe MCP tools to use.

---

## Problem Statement

MCP servers provide tools, resources, and notifications. Existing agents connect to them through config files at session startup. This creates three fundamental problems:

1. **Silent failures** — if a server can't start (runtime missing, auth required, timeout), the tools silently vanish from the tool list. The LLM never knows they existed.

2. **Static connections** — all servers must be pre-configured before the session starts. The agent can't dynamically connect to a new server mid-session based on what it's doing.

3. **Incompatible with skills** — skill files describe workflows that may require MCP servers. A skill that says "use the database MCP server" has no way to make that server available. The user must have pre-configured it.

Dynamic MCP solves all three by making connection a tool call.

---

## Current Architecture (What We Build On)

### 1. Bridge Tool & Relay System (`codelet/tools/src/bridge.rs`, `bridge_relay.rs`)

The Bridge tool provides WebSocket-based I/O relay:
- **Outbound**: Session `StreamChunk`s serialized as JSON to external WebSocket endpoints
- **Inbound**: Messages parsed and injected via `InputInjector` (`Arc<dyn Fn(InjectedInput) + Send + Sync>`)
- **Message types**: `input` (text/images), `control` (interrupt/clear/pause_response), `command`
- **Per-session context** via `BridgeSessionContext` with broadcast receivers, input injectors, and control handlers

### 2. Watcher Sessions (`session_manager.rs`)

Watcher sessions provide AI-mediated input injection:
- A watcher subscribes to a parent session's `broadcast` channel (receives all `StreamChunk`s)
- Interjections use `watcher_inject()` → `WatcherInput` → parent's `watcher_input_tx` channel
- The parent's `agent_loop` receives watcher input via `tokio::select!` alongside user input, with user input taking priority (`biased`)

### 3. Input Injection Channels

Every `BackgroundSession` has:
- `input_tx` / `input_rx`: Primary user input channel (`mpsc::Sender<PromptInput>`)
- `watcher_input_tx` / `watcher_input_rx`: Secondary injection channel for watchers/bridges (`mpsc::Sender<WatcherInput>`)
- `broadcast_tx`: Broadcast channel for outbound stream chunks

The `agent_loop` uses `tokio::select! { biased; }` to multiplex both channels, always preferring user input.

---

## Research: How Codex and OpenCode Implement MCP (Static MCP)

We examined the OpenAI Codex codebase (`/tmp/codex/codex-rs/`) and the OpenCode codebase (`/tmp/opencode-fresh/`) to understand how production MCP implementations work and where they fall short.

### Codex (Rust, `rmcp` crate)

- `McpConnectionManager` holds one `RmcpClient` per configured server
- Each connects via **stdio** (spawns child process) or **Streamable HTTP** (remote URL)
- Tools namespaced as `mcp__<server>__<tool>` and injected into the agent's tool list
- Startup: config → parallel `JoinSet` spawn → per-server handshake → `tools/list` → aggregate
- **~800 lines** of `McpConnectionManager` + `AsyncManagedClient` + startup event system + cancellation tokens
- Server-initiated messages handled via `LoggingClientHandler` trait callbacks — **logged but not surfaced to the LLM**
- Failures emit `McpStartupUpdateEvent` / `McpStartupCompleteEvent` — these go to the **TUI only** (confirmed: lines 5873-5874 of `codex.rs` explicitly exclude these from the agent loop)
- On failure: TUI flashes `"MCP startup incomplete (failed: filesystem)"` — **LLM sees nothing, tools silently vanish**

Codex's attempt at dynamic connections via `skill_dependencies.rs`:
1. Detect skills that declare MCP dependencies in metadata
2. Prompt user: "Install MCP servers? [Install] [Continue anyway]"
3. Write config entries to disk
4. Call `refresh_mcp_servers_now()` to hot-reload mid-turn

This only works for pre-declared dependencies with a clunky config-edit → prompt → refresh dance.

### OpenCode (TypeScript, `@modelcontextprotocol/sdk`)

- `MCP` namespace with `Instance.state()` lifecycle management
- Connects via `StdioClientTransport` (local) or `StreamableHTTPClientTransport`/`SSEClientTransport` (remote)
- Tools converted via `convertMcpTool()` → AI SDK `dynamicTool()` with proper schemas
- Startup: config → `Promise.all` across configured servers → per-server catch → status map
- **~970 lines** in `mcp/index.ts` + ~400 lines across auth files
- Status tracked as discriminated union: `connected | disabled | failed | needs_auth | needs_client_registration`
- Status is **for the TUI** — the LLM only sees whatever tools ended up in the tool list after init
- On failure: TUI toast `"Server requires authentication. Run: opencode mcp auth X"` — **LLM sees nothing**
- `ToolListChangedNotificationSchema` handler fires a bus event — TUI could react, **LLM doesn't know**
- `MCP.add(name, config)` exists as internal API but is **not exposed to the LLM** as a tool

### Common Failures in Static MCP (Both Codex and OpenCode)

| Failure | What Happens | What LLM Sees |
|---|---|---|
| Runtime not installed (`npx`, `python3`) | Process spawn fails → status: failed → TUI warning | Nothing — tools silently absent |
| Handshake timeout | Timeout error → status: failed → TUI warning | Nothing — tools silently absent |
| Auth required | Status: needs_auth → TUI toast/warning | Nothing — tools silently absent |
| Mid-session server crash | Internal status changes | Nothing — next tool call returns confusing error |
| Server sends notification | Logged internally / bus event | Nothing — not surfaced to LLM |

**The fundamental issue: connection management happens in a layer the LLM cannot observe.** All the complexity of startup orchestration, status tracking, and event systems exists precisely because the LLM isn't involved — so the system needs its own monitoring infrastructure. And despite all that infrastructure, the LLM still gets worse information than a single tool call would provide.

---

## Chosen Design: Dynamic MCP via `ConnectMCP` Tool

### Core Concept

MCP server connections are established through a `ConnectMCP` tool call. The agent decides when to connect, the tool handles the full MCP lifecycle (spawn, handshake, tool discovery), and returns a structured result. Connected servers' tools become available as callable tools immediately. Server-initiated messages are injected into the session.

```
Agent                          ConnectMCP Tool                    MCP Server
  │                                 │                                │
  │ ConnectMCP(name, command, env)  │                                │
  ├────────────────────────────────►│                                │
  │                                 │  spawn process                 │
  │                                 ├───────────────────────────────►│
  │                                 │  initialize handshake (MCP)    │
  │                                 │◄──────────────────────────────►│
  │                                 │  tools/list                    │
  │                                 │◄──────────────────────────────►│
  │  ✓ Connected: github            │                                │
  │  Tools: create_issue, list_repos│                                │
  │◄────────────────────────────────┤                                │
  │                                 │                                │
  │ mcp__github__create_issue(...)  │         tools/call             │
  ├────────────────────────────────►│───────────────────────────────►│
  │                                 │◄───────────────────────────────┤
  │  { content: [...] }             │                                │
  │◄────────────────────────────────┤                                │
  │                                 │                                │
  │                                 │  notification: tools_changed   │
  │                                 │◄───────────────────────────────┤
  │  [injected] MCP github:         │                                │
  │  tools list changed             │                                │
  │◄─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┤                                │
```

### How It Works

**1. Agent calls `ConnectMCP`:**

```
ConnectMCP(
  name: "github",
  transport: "stdio",
  command: "npx -y @modelcontextprotocol/server-github",
  env: { "GITHUB_TOKEN": "ghp_..." }
)
```

Or for remote servers:

```
ConnectMCP(
  name: "remote-db",
  transport: "http",
  url: "https://mcp.example.com/db",
  headers: { "Authorization": "Bearer ..." }
)
```

**2. Tool performs MCP handshake via rmcp crate:**

For stdio transport:
```rust
use rmcp::{ServiceExt, transport::{ConfigureCommandExt, TokioChildProcess}};
use tokio::process::Command;

// Build transport — TokioChildProcess spawns the child, pipes stdio
let transport = TokioChildProcess::new(
    Command::new("npx").configure(|cmd| {
        cmd.arg("-y").arg("@modelcontextprotocol/server-github");
        cmd.envs(env_vars);
    })
)?;

// serve_client does: send initialize request → receive ServerInfo → send initialized notification
let service = handler.serve(transport).await?;

// ServerInfo contains: name, version, capabilities (tools, resources, prompts, etc.)
let server_info = service.peer().peer_info();

// Fetch all tools (handles pagination automatically)
let tools: Vec<Tool> = service.peer().list_all_tools().await?;
```

For HTTP transport:
```rust
use rmcp::transport::StreamableHttpClientTransport;

let transport = StreamableHttpClientTransport::new(url);
let service = handler.serve(transport).await?;
```

The entire initialize → tools/list flow is a single `serve()` + `list_all_tools()` call. No manual JSON-RPC message construction needed.

**3. On success, returns structured result:**

```
✓ Connected: github (MCP 2025-06-18)
  Server: GitHub MCP Server v1.2.0
  Tools (3):
    - create_issue(owner, repo, title, body) — Create a new issue
    - list_repos(org) — List repositories for an organization
    - search_code(query, repo?) — Search code across repositories
  Resources: 2
```

**4. On failure, returns structured error:**

```
✗ Failed to connect: github
  npx: command not found
  
  Hint: This server requires Node.js. Install it or try an alternative.
```

```
✗ Failed to connect: github
  Timeout: MCP handshake not completed within 10s
  Process started (PID 48291) but did not respond.
```

```
✗ Failed to connect: remote-db
  HTTP 401 Unauthorized
  Server requires authentication.
```

**5. Connected server's tools appear in next LLM call:**

The connection is cached on the session as an ephemeral `McpConnection`:

```rust
struct McpConnection {
    // RunningService<RoleClient, DynMcpHandler> from rmcp crate.
    // Methods called via peer(): list_tools(), call_tool(), list_resources(), etc.
    // Server info available via peer().peer_info().
    service: RunningService<RoleClient, DynMcpHandler>,
    tools: Vec<Tool>,             // cached from peer().list_all_tools()
    name: String,
    server_info: ServerInfo,      // from initialize handshake
    connected_at: Instant,
    call_count: u32,
}
```

The rmcp `Tool` struct contains: `name` (Cow<str>), `title` (Option<String>), `description` (Option<Cow<str>>), `input_schema` (Arc<JsonObject>), `output_schema` (Option<Arc<JsonObject>>), `annotations` (Option<ToolAnnotations>).

Session holds: `mcp_connections: HashMap<String, McpConnection>`

When building the tool list for the next LLM API call:

```
built_in_tools + session.mcp_connections.values().flat_map(|c| c.tools)
```

No registry. No lifecycle manager. The session *is* the scope.

**6. MCP tool calls are routed through the connection:**

When the LLM calls `mcp__github__create_issue(...)`:
- Split on `__` → find `"github"` in `session.mcp_connections`
- Forward via rmcp's `Peer<RoleClient>`:
  ```rust
  let result = connection.service.peer()
      .call_tool(CallToolRequestParams::new("create_issue").with_arguments(args))
      .await?;
  // result: CallToolResult { content: Vec<Content>, is_error: Option<bool>, ... }
  ```
- Return content from `CallToolResult` as tool output to the LLM

**7. Server-initiated messages are injected into the session:**

Implement rmcp's `ClientHandler` trait on `DynMcpHandler`. The trait provides default no-op implementations for all callbacks — we override the ones we need:

- `on_tool_list_changed(context)` → re-fetch `peer().list_all_tools()`, update cached tools, inject notification
- `on_resource_updated(params, context)` → inject notification with `params.uri`
- `on_resource_list_changed(context)` → inject notification
- `create_message(params, context)` → inject `CreateMessageRequestParams` as session input for the LLM to respond to, return `CreateMessageResult` back to server
- `on_logging_message(params, context)` → inject log message as informational

All injected via `watcher_input_tx`, same path as watcher sessions and Bridge relay.

**8. Session ends → connections cleaned up:**

Drop all `McpConnection`s → `RunningService::drop()` triggers cancellation → rmcp's `ChildWithCleanup::drop()` calls `child.kill()` on stdio transport processes → done. No additional cleanup infrastructure needed beyond what rmcp provides.

### Skill-Driven Dynamic MCP

This is where Dynamic MCP fundamentally differs from Static MCP. Skills can describe MCP tools to connect to, and the agent connects them as part of executing the skill:

```markdown
<!-- skill: code-review.md -->
# Code Review Skill

## Setup
Connect to the GitHub MCP server for repository access:
- ConnectMCP(name: "github", transport: "stdio", command: "npx -y @modelcontextprotocol/server-github", env: { "GITHUB_TOKEN": "$GITHUB_TOKEN" })

Connect to the SonarQube MCP server for code quality analysis:
- ConnectMCP(name: "sonar", transport: "http", url: "https://sonar.internal/mcp")

## Workflow
1. Use `mcp__github__get_pull_request` to fetch the PR details
2. Use `mcp__github__list_pr_files` to get changed files
3. Use `mcp__sonar__analyze` to run quality checks on each file
4. Compile findings and post review via `mcp__github__create_review`
```

Agent execution:
1. Agent reads skill
2. Calls `ConnectMCP(name: "github", ...)` → success, 5 tools available
3. Calls `ConnectMCP(name: "sonar", ...)` → success, 3 tools available
4. Now has 8 MCP tools available alongside built-in tools
5. Executes workflow using typed MCP tool calls
6. Session ends → both connections cleaned up

**This is impossible with Static MCP.** In Codex, the skill would need to edit a TOML config file and trigger a hot-reload. In OpenCode, skills have no mechanism to bring MCP servers online at all. With Dynamic MCP, skills that use multiple MCP servers just work — the agent connects to each one as it goes.

### DisconnectMCP

```
ConnectMCP(action: "disconnect", name: "github")
```

Or as a separate tool:

```
DisconnectMCP(name: "github")
```

Returns:
```
✓ Disconnected: github (was connected 5m, 12 tool calls made)
```

Useful when a skill is done with a server and wants to clean up, or when the agent wants to free resources.

### ListMCP

```
ConnectMCP(action: "list")
```

Returns:
```
Connected MCP servers:
  github — connected 5m ago, 5 tools, 12 calls
  sonar — connected 2m ago, 3 tools, 0 calls
```

---

## Dynamic MCP vs Static MCP: Full Comparison

### Connection Lifecycle

| | Codex (Static) | OpenCode (Static) | Dynamic MCP |
|---|---|---|---|
| **When** | Before session starts | Before session starts | When agent decides |
| **How** | Config → parallel JoinSet | Config → Promise.all | `ConnectMCP(...)` tool call |
| **Who decides** | User (via config file) | User (via config file) | Agent (via reasoning or skill) |
| **Orchestration** | ~800 lines McpConnectionManager | ~970 lines MCP namespace | Tool handler + ephemeral session cache |

### Failure Visibility

| Failure | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| Runtime missing | TUI warning flash → **LLM: nothing** | TUI status: failed → **LLM: nothing** | Tool result: `✗ npx: command not found` → **LLM reasons about it** |
| Handshake timeout | TUI warning → **LLM: nothing** | TUI status: failed → **LLM: nothing** | Tool result: `✗ Timeout after 10s` → **LLM can retry or suggest fix** |
| Auth required | TUI warning → **LLM: nothing** | TUI toast → **LLM: nothing** | Tool result: `✗ 401 Unauthorized` → **LLM tells user what to do** |
| Mid-session crash | **Nobody knows** until tool call fails | **LLM doesn't know** | Notification injected: `[MCP:github] Disconnected` → **LLM knows immediately** |
| Server not configured | Tools silently absent | Tools silently absent | Not applicable — no config needed |

### Dynamic Connections

| | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| Mid-session connect | Config edit → refresh (clunky) | `MCP.add()` internal API only | `ConnectMCP(...)` tool call |
| Skill-driven | Limited: skill_dependencies metadata detection | No mechanism | Natural: skill describes ConnectMCP calls, agent executes them |
| Multiple servers per skill | Requires all pre-configured | Requires all pre-configured | Skill connects each as needed |
| Context-driven | No — config is static | No — config is static | Yes — agent decides based on what it's doing |

### Tool Integration

| | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| Tool schemas | ✅ Native typed tools | ✅ Native typed tools | ✅ Native typed tools (same `rmcp` crate) |
| Tool discovery | Implicit (appear in list) | Implicit (appear in list) | Explicit (ConnectMCP result lists them) + implicit (appear in list) |
| Tool namespacing | `mcp__server__tool` | `server_tool` | `mcp__server__tool` |
| Tool disappearance | Silent | Silent | Notification injected |

### Server-Initiated Messages

| | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| `tools/list_changed` | `LoggingClientHandler` logs it | Bus event for TUI | Re-fetch tools + inject notification → **LLM knows** |
| `resources/updated` | Logged | Not handled | Inject into session → **LLM knows** |
| `sampling/createMessage` | Handler callback (unclear if surfaced) | Not implemented | Inject as session input → **LLM responds** |
| General notifications | Internal callbacks only | Internal callbacks only | Surfaced to LLM via `watcher_input_tx` |

### Implementation Complexity

| | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| Connection management | `McpConnectionManager` (~800 lines), `AsyncManagedClient`, startup events, cancellation tokens, `ProcessGroupGuard` | `MCP` namespace (~970 lines), state machine, status tracking | `ConnectMCP` tool handler (~150 lines), `McpConnection` struct on session |
| Config system | TOML schema, validation, skill_dependencies | Zod schema, discriminated unions | None required (optional config-as-sugar later) |
| Startup orchestration | JoinSet parallel spawn, per-server status, completion event aggregation | Promise.all with per-server catch, status map | None — agent calls tool sequentially |
| Auth | OAuth via rmcp SDK | Full OAuth flow (~400 lines: provider, callback server, browser, state) | Delegate to `rmcp` built-in auth. Clear error on failure |
| Process cleanup | `ProcessGroupGuard` with SIGTERM → grace → SIGKILL, descendant tree walking | Manual `descendants()` pgrep walking | Session drop kills child processes |
| **Total MCP code** | **~1500+ lines** | **~1400+ lines** | **~300 lines estimated** |

### User Experience

| | Codex | OpenCode | Dynamic MCP |
|---|---|---|---|
| Setup effort | Edit config file, list commands + args + env | Edit config file, list commands + args + env | None — agent handles it, or skill describes it |
| "It doesn't work" debugging | Check TUI warning (gone in 2s), check logs, wonder why tools aren't there | Check TUI status, run CLI auth command | Agent tells you in conversation what happened |
| Adding a new MCP server | Edit config, restart session | Edit config, restart | Tell the agent, or use a skill that includes it |
| LLM token cost | Zero — happens before LLM | Zero — happens before LLM | One tool call per connection (~minimal) |

### The Trade-Off

Dynamic MCP costs one tool call turn per connection. Static MCP costs zero LLM tokens.

But avoiding that one tool call is what causes every problem with Static MCP: because connection happens before the agent, the agent can't see failures, can't decide dynamically, can't be driven by skills. Codex and OpenCode built ~1500 lines of infrastructure (connection managers, startup orchestration, event systems, status tracking, TUI integration) to manage what Dynamic MCP handles with a tool call and a struct on the session.

The token cost is negligible. The architecture cost of avoiding it is enormous and delivers a worse experience.

---

## Implementation Scope

### 1. `McpConnection` Struct

Ephemeral, lives on the session:

```rust
use rmcp::{
    ClientHandler,
    model::{Tool, ServerInfo, CallToolRequestParams, CallToolResult,
            CreateMessageRequestParams, CreateMessageResult,
            LoggingMessageNotificationParam, ResourceUpdatedNotificationParam},
    service::{RunningService, RoleClient, RequestContext, NotificationContext},
    transport::TokioChildProcess,
};

struct McpConnection {
    service: RunningService<RoleClient, DynMcpHandler>,
    tools: Vec<Tool>,             // cached from peer().list_all_tools()
    name: String,
    server_info: ServerInfo,
    connected_at: Instant,
    call_count: u32,
}

// DynMcpHandler implements ClientHandler trait
// Holds Arc references to session injection channels
struct DynMcpHandler {
    name: String,
    watcher_input_tx: mpsc::Sender<WatcherInput>,
    tools_cache: Arc<RwLock<Vec<Tool>>>,
}

impl ClientHandler for DynMcpHandler {
    // Override on_tool_list_changed, on_resource_updated, 
    // create_message, on_logging_message
    // Default no-op implementations for everything else
}
```

Session holds: `mcp_connections: HashMap<String, McpConnection>`

### 2. `ConnectMCP` Tool

Actions:
- **`connect`** (default) — create transport (TokioChildProcess for stdio, StreamableHttpClientTransport for HTTP), call `handler.serve(transport)` which does the full initialize handshake, then `peer().list_all_tools()`, cache on session, return result
- **`disconnect`** — call `service.cancel()` which triggers graceful shutdown and process cleanup, remove from session, return confirmation
- **`list`** — return status of all active connections

Parameters for connect:
- `name` (required) — identifier for this connection
- `transport` — `"stdio"` or `"http"`
- `command` — command to spawn (stdio transport)
- `url` — server URL (http transport)
- `env` — environment variables for the subprocess
- `headers` — HTTP headers (http transport)
- `timeout` — connection timeout in seconds (default: 10)

### 3. MCP Tool Call Routing

When the LLM calls `mcp__<server>__<tool>(args)`:
1. Parse server name and tool name from the qualified name
2. Look up `server` in `session.mcp_connections`
3. Call `connection.service.peer().call_tool(CallToolRequestParams::new(tool).with_arguments(args))`
4. Receive `CallToolResult { content: Vec<Content>, is_error: Option<bool>, .. }`
5. Return content to LLM as tool output

### 4. `ClientHandler` Implementation for Notifications

Implement rmcp's `ClientHandler` trait on `DynMcpHandler`:

```rust
impl ClientHandler for DynMcpHandler {
    // Server asks client to run LLM sampling
    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, McpError> {
        // Inject into session via watcher_input_tx
        // Wait for LLM response
        // Return as CreateMessageResult
    }

    // Server's tool list changed
    async fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) {
        // Re-fetch: context.peer.list_all_tools().await
        // Update cached tools
        // Inject notification into session
    }

    // A subscribed resource was updated
    async fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        // Inject "[MCP:<name>] Resource updated: <params.uri>" into session
    }

    // Server sent a log message
    async fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        context: NotificationContext<RoleClient>,
    ) {
        // Inject "[MCP:<name>] Log (<params.level>): <params.data>" into session
    }

    fn get_info(&self) -> ClientInfo {
        ClientInfo {
            name: "codelet".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ..Default::default()
        }
    }
}
```

All injected via `watcher_input_tx`.

### 5. Tool List Assembly

Each LLM API call already gathers tools fresh:

```rust
fn gather_tools(session: &Session) -> Vec<Tool> {
    let mut tools = built_in_tools();
    for conn in session.mcp_connections.values() {
        for tool in &conn.tools {
            let qualified_name = format!("mcp__{}__{}", conn.name, tool.name);
            tools.push(tool.with_name(qualified_name));
        }
    }
    tools
}
```

No registry. No event system. Just read the HashMap.

### 6. Session Cleanup

When session ends, drop `mcp_connections` → each `RunningService` drop triggers its cancellation token → rmcp's internal service loop exits with `QuitReason::Cancelled` → transport `close()` is called → for stdio transport, `ChildWithCleanup::drop()` spawns an async `child.kill()`. No additional cleanup code needed beyond what rmcp provides.

---

## Resolved Questions

| Question | Decision | Rationale |
|---|---|---|
| Tool or config-driven? | **Tool (`ConnectMCP`)** | Agent sees failures, enables dynamic connections, compatible with skills |
| Use `rmcp` crate? | **Yes** | Same crate as Codex. Production-quality MCP protocol implementation |
| New tool or extend Bridge? | **New `ConnectMCP` tool** | Semantically distinct from Bridge (WebSocket relay). Clean separation |
| Support stdio + HTTP transports? | **Yes, both** | stdio for local servers, HTTP for remote. Same as Codex/OpenCode |
| Auth handling? | **Delegate to `rmcp` built-in auth** for V1 | Return clear error if auth fails. Full OAuth flow is V2 |
| Dynamic tool list? | **Ephemeral cache on session** | Gathered fresh each LLM call. `tools/list_changed` updates the cache |
| Server notifications surfaced? | **Yes, via `watcher_input_tx`** | Same injection path as watchers and Bridge |
| Config file support? | **V2 — optional sugar** | Agent reads config, calls ConnectMCP for each. Tool is the primitive |
| Multiple connections? | **Yes** | Each ConnectMCP call adds to session's HashMap. Independent lifecycles |
| Process cleanup? | **Session drop** | Kill child processes when session ends. No separate lifecycle manager |

---

## Rejected Options (With Rationale)

### Option A: WebSocket Listener with Proxy Scripts (Previous Design)

Auto-start WebSocket listener per session, user writes proxy scripts that bridge between MCP servers and the listener. Rejected because:
- **User writes code** — proxy scripts are friction; ConnectMCP needs zero user code for standard MCP servers
- **No typed tools** — proxy scripts inject text, not structured tool schemas
- **Failure visibility was better than Static MCP but worse than ConnectMCP** — Bash stderr is unstructured; ConnectMCP returns a typed tool result
- **Still useful for non-MCP integrations** — may revisit as a separate, smaller story for custom protocol bridging

### Option B: Named Pipe / Unix Socket Listener

Unix domain socket at `/tmp/fspec-sessions/<session-id>.sock`. Rejected because:
- Not cross-platform (Windows needs different API)
- Less tooling support than WebSocket
- No built-in response channel without stream sockets
- Superseded by ConnectMCP approach entirely

### Option C: Static MCP via Config (What Codex/OpenCode Do)

Pre-configure servers in config file, connect at startup. Rejected because:
- **Silent failures** — LLM never knows when connections fail
- **No dynamic connections** — can't connect mid-session based on context
- **Incompatible with skills** — skills can't bring MCP servers online
- **More code** — ~1500 lines of orchestration infrastructure vs ~300 lines for ConnectMCP
- See detailed comparison above

### Option D: Subprocess Listener with stdio Relay

Agent spawns proxy script as subprocess, relays stdout/stdin. Rejected because:
- Only one MCP server per subprocess
- No reconnection semantics
- No typed tool schemas
- Superseded by ConnectMCP which speaks MCP natively

### Option E: Hybrid with AI Mediation Layer

Route server-initiated messages through a watcher-like AI evaluator before injection. Rejected for V1 because:
- Much more complex
- Uses LLM tokens for filtering
- Latency for urgent messages
- Could be added as V2 enhancement if notification noise becomes a problem
