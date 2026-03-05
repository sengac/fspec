# MCP-001 Implementation Status

## Last Updated: 2026-03-05

## Current Phase: IMPLEMENTING (13 story points)

---

## What's Complete ✅

### 1. Core Module (`codelet/tools/src/mcp.rs` — 2,474 lines)

**Production code (~1,077 lines):**

- `McpConnection` — holds `RunningService<RoleClient, DynMcpHandler>`, cached tool lists (simplified + raw rmcp `Tool`s), `McpServerInfo`, connection stats (uptime, call count), transport type
- `McpToolDef` — simplified tool definition for serialization to LLM
- `McpServerInfo` — server name, version, negotiated protocol version
- `McpTransport` — `Stdio | Http` enum with JsonSchema
- `McpAction` — `Connect | Disconnect | List` enum with default `Connect`
- `McpConnectArgs` — full tool arguments with JsonSchema derivation for LLM schema generation. Fields: action, name, transport, command, url, env, headers, timeout
- `McpConnectResult` / `McpConnectionSummary` — structured result types
- `McpInjection` enum — `Notification(String)` and `SamplingRequest { params, response_tx }` for oneshot round-trip
- `McpInjectionTx` — `mpsc::Sender<McpInjection>` type alias
- `McpConnectionMap` — `Arc<RwLock<HashMap<String, McpConnection>>>` type alias

**ClientHandler (`DynMcpHandler`):**

- `get_info()` — returns `ClientInfo` with "codelet" name and crate version
- `create_message()` — sampling/createMessage round-trip via oneshot channel: creates `oneshot::channel`, sends `McpInjection::SamplingRequest` through `injection_tx`, awaits response on `oneshot_rx`, returns `CreateMessageResult` to rmcp
- `on_tool_list_changed()` — re-fetches `peer().list_all_tools()`, updates both `tools` and `raw_tools` in the connection map, injects notification via `injection_tx`
- `on_resource_updated()` — injects `[MCP:<name>] Resource updated: <uri>` notification
- `on_logging_message()` — injects `[MCP:<name>] Log (<level>): <data>` notification

**Connection Functions:**

- `connect_stdio()` — parses command string, creates `TokioChildProcess` transport via `ConfigureCommandExt`, optional env vars, handshake with configurable timeout, `list_all_tools()`, caches connection. Handles: empty command, spawn failure (with hint about PATH), handshake failure, timeout
- `connect_http()` — creates `StreamableHttpClientTransport::from_uri()`, same handshake flow. Custom headers logged as unsupported for V1 (TODO). Handles: 401/Unauthorized with auth hint, timeout
- `disconnect_mcp()` — removes from map, cancels service via `cancellation_token().cancel()`, returns stats (uptime, call count)
- `route_mcp_tool_call()` — parses `mcp__<server>__<tool>`, looks up connection, increments call count, calls `peer().call_tool(CallToolRequestParams)`, handles error responses and content extraction

**Global Per-Session State (`MCP_SESSIONS` — `once_cell::sync::Lazy<std::sync::Mutex<HashMap<Uuid, McpSessionState>>>`):**

- `init_mcp_session()` — creates `mpsc::channel::<McpInjection>(32)`, creates `McpConnectionMap`, stores in global map, returns `(injection_rx, connections)`
- `cleanup_mcp_session()` — removes from global map, cancels all connections synchronously via `try_write()` + `cancellation_token().cancel()`
- `get_mcp_connections()` — returns `Option<McpConnectionMap>` for agent building
- `get_mcp_session_state()` — returns `(McpInjectionTx, McpConnectionMap)` for `ConnectMcpTool`

**Agent Integration:**

- `McpToolRegistration` — holds namespaced `rmcp::model::Tool` + `ServerSink` for rig's `rmcp_tool()` builder
- `gather_mcp_tool_registrations()` — async, reads connections, namespaces tools, returns `Vec<McpToolRegistration>`
- `McpToolWrapper` — implements `rig::tool::Tool` trait for individual MCP server tools. Uses dummy `const NAME` + overrides `fn name()` for dynamic qualified names. `call()` routes through `route_mcp_tool_call()`
- `gather_mcp_tool_wrappers()` — sync, uses `try_read()` (non-blocking) so MCP tools appear next turn if lock is briefly held

**ConnectMcpTool (`rig::tool::Tool` implementation):**

- `const NAME = "ConnectMCP"`
- `definition()` — returns description + JsonSchema parameters
- `call()` — validates args per action, dispatches to `connect_stdio()`, `connect_http()`, `disconnect_mcp()`, or `list_connections()`. Returns formatted `McpConnectResult` as JSON string

**Helper Functions:**

- `parse_mcp_tool_name()` — splits `mcp__<server>__<tool>` into `(server, tool)`
- `qualified_mcp_tool_name()` — builds `mcp__<server>__<tool>` from parts
- `gather_mcp_tools()` — namespaced tool list from connections map
- `list_connections()` — builds `Vec<McpConnectionSummary>` with uptime/stats
- `format_uptime()` — human-readable elapsed time

**Tests (~1,300 lines, 56 tests, all passing):**

All 15 feature scenarios have corresponding test functions plus helper tests:
1. `test_connect_to_mcp_server_via_stdio_transport`
2. `test_route_tool_call_through_cached_mcp_connection`
3. `test_handle_spawn_failure_with_structured_error`
4. `test_connect_to_mcp_server_via_http_transport`
5. `test_multi_server_workflow_with_independent_connections`
6. `test_receive_server_tool_list_changed_notification`
7. `test_handle_server_sampling_create_message_request`
8. `test_list_active_mcp_connections`
9. `test_disconnect_an_active_mcp_connection`
10. `test_handle_connection_timeout_during_initialization`
11. `test_handle_http_authentication_failure`
12. `test_session_cleanup_kills_all_mcp_connections`
13. `test_tool_list_assembly_includes_mcp_tools_alongside_built_in_tools`
14. `test_tool_name_collision_prevented_by_double_underscore_namespacing`
15. `test_connect_mcp_with_duplicate_server_name_replaces_existing`
16. `test_mcp_connect_args_with_env` (helper)
17. `test_mcp_connect_args_with_headers` (helper)
Plus: `test_gather_mcp_tool_registrations_empty_session`, `test_gather_mcp_tool_registrations_nonexistent_session`, and more

All tests use mocks (mock rmcp services via test helpers). No real MCP servers required.

### 2. Module Registration (`codelet/tools/src/lib.rs`)

- `pub mod mcp;` added
- All public types and functions re-exported: `ConnectMcpTool`, `McpConnection`, `McpConnectionMap`, `McpInjection`, `McpInjectionTx`, `DynMcpHandler`, `McpToolWrapper`, `McpToolRegistration`, `init_mcp_session`, `cleanup_mcp_session`, `gather_mcp_tool_wrappers`, `gather_mcp_tool_registrations`, `get_mcp_connections`, `connect_stdio`, `connect_http`, `disconnect_mcp`, `route_mcp_tool_call`, `parse_mcp_tool_name`, `qualified_mcp_tool_name`, `new_mcp_connection_map`, plus all data types

### 3. Dependencies (`codelet/tools/Cargo.toml`, `codelet/Cargo.toml`)

- `rmcp` added with `client`, `transport-child-process`, `transport-streamable-http-client-reqwest` features
- Workspace dependency configured

### 4. Provider Integration (all 5 providers)

Each provider's `create_rig_agent()` now includes `.tool(ConnectMcpTool::new(session_id))`:

- `codelet/providers/src/claude.rs` ✅
- `codelet/providers/src/openai.rs` ✅
- `codelet/providers/src/gemini.rs` ✅
- `codelet/providers/src/codex/mod.rs` ✅
- `codelet/providers/src/zai.rs` ✅

### 5. Session Lifecycle (`codelet/napi/src/session_manager.rs`)

- **Session creation** (both regular and isolated): `init_mcp_session(uuid)` called after `BackgroundSession::new()`, returns `(_mcp_injection_rx, _mcp_connections)`
- **Session close**: `cleanup_mcp_session(uuid)` called before dropping the session
- **Agent building** (`run_with_provider!` macro): Each turn calls `gather_mcp_tool_wrappers(session.id)`, then adds each `McpToolWrapper` to the agent via `agent.tool_server_handle.add_tool(wrapper).await`

### 6. Feature File & Coverage

- `spec/features/dynamic-mcp-connect.feature` — 15 scenarios, tagged `@wip @MCP-001`
- `spec/features/dynamic-mcp-connect.feature.coverage` — 100% (15/15 scenarios linked to tests and implementation)

### 7. Build & Test Status

- `cargo check` — entire workspace compiles clean
- `cargo test -p codelet-tools --lib -- mcp` — 56/56 tests pass
- `cargo test -p codelet-tools` — 479/479 tests pass (no regressions)

---

## What's Remaining 🔲

### 1. ~~MCP Injection Channel Not Wired Into Agent Loop~~ ✅ DONE

**Status**: Implemented via **Option A** — `mcp_injection_rx` is passed as a third parameter to `agent_loop` and consumed in a third `tokio::select!` branch.

**What's wired:**
- **`McpInjection::Notification(text)`** — emitted as a `watcher_input` StreamChunk for UI display, then processed as LLM input so the agent can react to server notifications (tool list changes, resource updates, logging messages)
- **`McpInjection::SamplingRequest { params, response_tx }`** — returns a structured error through `response_tx` ("sampling/createMessage not yet supported — V2 feature"). The MCP server receives a proper error response instead of a hanging channel. Marked as V2 scope.

**Changes:**
- `session_manager.rs`: `agent_loop()` signature extended with `mut mcp_injection_rx: mpsc::Receiver<McpInjection>`
- `session_manager.rs`: Both `create_session_with_id()` and `create_isolated_session_with_id()` pass `mcp_injection_rx` to `agent_loop()`
- `session_manager.rs`: Added `use codelet_tools::McpInjection;` import
- `tokio::select!` now has 3 biased branches: user input > watcher input > MCP injection

### 2. Custom HTTP Headers Not Implemented

**Status**: `connect_http()` logs a warning and ignores the `headers` parameter. The TODO at line 677 explains: building with a custom `reqwest::Client` requires resolving reqwest version alignment between rmcp and codelet-tools.

**Impact**: MCP servers requiring auth via HTTP headers won't work until this is addressed. The `env` parameter for stdio transport works fine (used for API tokens in env vars).

**Priority**: Low for V1 — most MCP servers use stdio transport with env vars for auth. HTTP servers requiring custom headers are uncommon.

### 3. No End-to-End Integration Test With Real MCP Server

**Status**: All 56 tests use mocks. There is no test that spawns an actual MCP server (e.g., `npx -y @modelcontextprotocol/server-filesystem`) and verifies the full connect → tools/list → tool call → disconnect flow.

**Priority**: Nice-to-have for V1. The mock tests comprehensively cover all code paths. A real server test would catch transport/protocol edge cases but requires a runtime (Node.js for npx) in CI.

### 4. Nothing Committed Yet

All changes are unstaged. The full diff covers:
- `codelet/Cargo.lock` (243 lines, rmcp dependencies)
- `codelet/Cargo.toml` (3 lines, workspace deps)
- `codelet/tools/Cargo.toml` (3 lines, rmcp dep)
- `codelet/tools/src/lib.rs` (10 lines, module + exports)
- `codelet/tools/src/mcp.rs` (2,474 lines, NEW file)
- `codelet/providers/src/*.rs` (5 files, ~5 lines each)
- `codelet/napi/src/session_manager.rs` (~50 lines)
- `spec/features/dynamic-mcp-connect.feature` (NEW)
- `spec/features/dynamic-mcp-connect.feature.coverage` (NEW)
- `spec/attachments/MCP-001/*` (updated)
- `spec/work-units.json` (updated)

---

## Architecture Decisions Made

1. **Global per-session state via `once_cell::Lazy<std::sync::Mutex<HashMap<Uuid, McpSessionState>>>`** — avoids threading `McpConnectionMap` through `BackgroundSession` (which would require changes to many call sites). The `ConnectMcpTool` looks up its session's state by UUID. `std::sync::Mutex` chosen over `tokio::sync::Mutex` because we never hold the lock across `.await` points.

2. **`McpToolWrapper` implements `rig::tool::Tool`** — each MCP server tool gets its own wrapper instance that routes calls through `route_mcp_tool_call()`. This integrates with rig's `tool_server_handle.add_tool()` post-build method, so MCP tools are added after the agent is built (not during `create_rig_agent()`).

3. **Non-blocking tool gathering via `try_read()`** — `gather_mcp_tool_wrappers()` uses `RwLock::try_read()` so that if a connect/disconnect is in progress (holding write lock), the agent build doesn't block. MCP tools simply appear on the next turn.

4. **Separate injection channel (not watcher_input_tx)** — `DynMcpHandler` sends `McpInjection` messages through a dedicated `mpsc::channel`, not directly into `watcher_input_tx`. This keeps the MCP module independent of session_manager types and allows typed discrimination between notifications and sampling requests.

5. **Service cancellation via `cancellation_token().cancel()`** — cleanup is synchronous (cancel is a sync operation on the token). Child processes are killed by rmcp's internal `ChildWithCleanup::drop()` which spawns `child.kill()`.

---

## File Map

| File | Lines | Status |
|------|-------|--------|
| `codelet/tools/src/mcp.rs` | 2,474 | NEW (unstaged) |
| `codelet/tools/src/lib.rs` | +10 | Modified (unstaged) |
| `codelet/tools/Cargo.toml` | +3 | Modified (unstaged) |
| `codelet/Cargo.toml` | +3 | Modified (unstaged) |
| `codelet/Cargo.lock` | +243 | Modified (unstaged) |
| `codelet/providers/src/claude.rs` | +2 | Modified (unstaged) |
| `codelet/providers/src/openai.rs` | +2 | Modified (unstaged) |
| `codelet/providers/src/gemini.rs` | +2 | Modified (unstaged) |
| `codelet/providers/src/codex/mod.rs` | +2 | Modified (unstaged) |
| `codelet/providers/src/zai.rs` | +2 | Modified (unstaged) |
| `codelet/napi/src/session_manager.rs` | +50 | Modified (unstaged) |
| `spec/features/dynamic-mcp-connect.feature` | ~180 | NEW (untracked) |
| `spec/features/dynamic-mcp-connect.feature.coverage` | ~200 | NEW (untracked) |

---

## Next Steps (Recommended Order)

1. ~~**Wire MCP injection channel**~~ ✅ Done (Option A — third `tokio::select!` branch)
2. **Validate** — Run `fspec validate`, `fspec validate-tags`, `fspec show-coverage`, full test suite.
3. **Move to validating** → then **done**.
4. **Commit** all changes.
