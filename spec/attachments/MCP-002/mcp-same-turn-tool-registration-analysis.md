# MCP-002: MCP Tools Not Available in Same Turn as ConnectMCP Call

## Summary

When an agent calls `ConnectMCP` to establish a connection to an MCP server (e.g., Playwright), the newly discovered tools (e.g., `mcp__playwright__browser_navigate`) are **not callable until the next turn**. The agent receives "Tool not found" errors for any same-turn tool calls to the newly connected MCP server.

## Reproduction Steps

1. Agent calls `ConnectMCP` with `name: "playwright"`, `transport: "stdio"`, `command: "npx @playwright/mcp@latest"`
2. Connection succeeds — response lists 22 tools
3. Agent immediately calls `mcp__playwright__browser_navigate` in the same turn
4. **Result**: "Tool not found"
5. User replies (triggering a new turn)
6. Agent calls `mcp__playwright__browser_navigate` again
7. **Result**: Works perfectly

## Root Cause Analysis

### Agent Turn Lifecycle

The `run_with_provider!` macro in `codelet/napi/src/session_manager.rs` (lines 4966-5017) executes this sequence per turn:

```
1. gather_mcp_tool_wrappers(session.id)  ← collects EXISTING MCP tools from McpConnectionMap
2. provider.create_rig_agent(...)         ← builds agent with built-in tools (Read, Write, Bash, etc.)
3. agent.tool_server_handle.add_tool()    ← adds MCP wrappers gathered in step 1
4. run agent stream                       ← agent runs; LLM may call ConnectMCP HERE
```

When `ConnectMCP` succeeds during step 4:
- The connection is cached in `McpConnectionMap` (global per-session state)
- Tool definitions are stored in `McpConnection.tools` and `McpConnection.raw_tools`
- **But** the running agent's tool list was frozen in steps 1-3
- The newly connected tools are **never added to `tool_server_handle`** mid-turn

### Key Files

| File | Role |
|------|------|
| `codelet/napi/src/session_manager.rs:4966-5017` | `run_with_provider!` macro — assembles tools per turn |
| `codelet/tools/src/mcp.rs:989-1016` | `gather_mcp_tool_wrappers()` — reads McpConnectionMap |
| `codelet/tools/src/mcp.rs:1038-1170` | `ConnectMcpTool::call()` — establishes connection |
| `codelet/tools/src/mcp.rs:934-981` | `McpToolWrapper` — rig::tool::Tool for each MCP tool |
| `codelet/tools/src/mcp.rs:480-503` | `connect_stdio()` — caches connection in map |

### Existing Infrastructure

The `tool_server_handle.add_tool()` mechanism **already exists** and works — it's used in step 3 of the macro to add MCP tools that were connected in a *previous* turn. The mechanism just isn't invoked after a successful `ConnectMCP` within the *current* turn.

## Proposed Fix

### Option A: ConnectMcpTool Registers Tools Directly (Recommended)

Give `ConnectMcpTool` access to the running agent's `ToolServerHandle` so it can call `add_tool()` for each newly discovered tool immediately after a successful connection.

**Implementation steps:**

1. **Store `ToolServerHandle` in per-session MCP state** — When the agent is built in `run_with_provider!`, clone the handle and store it alongside the `McpConnectionMap` in the global MCP session state.

2. **After successful connect in `ConnectMcpTool::call()`** — Build `McpToolWrapper` instances for each discovered tool and call `handle.add_tool(wrapper)` for each one.

3. **Handle the async nature** — `add_tool()` is async, and `ConnectMcpTool::call()` is already async, so this fits naturally.

**Rough code sketch for `ConnectMcpTool::call()` after successful connect:**
```rust
// After successful connect, register tools with running agent
if result.success {
    if let Some(handle) = get_tool_server_handle(self.session_id) {
        let wrappers = gather_mcp_tool_wrappers(self.session_id);
        for wrapper in wrappers {
            // Only add newly connected tools (filter by server name)
            if wrapper.qualified_name.starts_with(&format!("mcp__{name}__")) {
                let _ = handle.add_tool(wrapper).await;
            }
        }
    }
}
```

### Option B: Post-Tool-Call Hook

Add a mechanism where after any tool call completes, the agent loop checks if new MCP tools need to be registered. This is more generic but adds overhead to every tool call.

### Option C: Two-Phase ConnectMCP

Split ConnectMCP into `connect` (establishes connection) and the tools only become available after the agent loop detects the change. This is essentially the current behavior, just documented. **Not recommended** — poor UX.

## Recommendation

**Option A** is the cleanest fix. The `ToolServerHandle` already supports dynamic tool addition via `add_tool()`. The only missing piece is making the handle accessible from within `ConnectMcpTool::call()`.

## Test Strategy

1. **Unit test**: After `ConnectMcpTool::call()` succeeds, verify that `tool_server_handle` contains the new MCP tool wrappers
2. **Integration test**: Connect to an MCP server and immediately call one of its tools in the same turn — should succeed without "Tool not found"
