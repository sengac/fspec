# RPC-083 AST research — tool registry call sites in agent-loop

Goal: Confirm that the canonical "tool registry via `create_rig_agent` +
`RigAgent::with_default_depth` + MCP wrapper gather + `set_mcp_tool_server_handle`"
plumbing is already present in the NAPI-free `codelet-agent-loop` crate so this
work unit is **coverage-only** (mirrors RPC-082 path).

## Findings

### 1. `run_with_provider!` macro — `codelet/agent-loop/src/dispatch.rs:37-106`

Five required calls are present in the macro body verbatim from NAPI:

| # | Call | Line | Purpose |
|---|------|------|---------|
| 1 | `codelet_tools::gather_mcp_tool_wrappers($session.id)` | 51 | MCP-001 — collect previous-turn MCP tools |
| 2 | `provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone())` | 58-62 | Per-provider tool registry (~21 tools) + role preamble + thinking config |
| 3 | `agent.tool_server_handle.add_tool(wrapper).await` (loop) | 72-76 | Install MCP wrappers on the rig agent |
| 4 | `codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone())` | 81-84 | Make handle visible to ConnectMcpTool for mid-turn tools |
| 5 | `codelet_core::RigAgent::with_default_depth(agent)` | 86 | Wrap with inner agentic loop (bounded depth) |

### 2. Provider arms calling the macro — `codelet/agent-loop/src/agent_loop.rs:867-952`

- `:868` — `claude` arm via `run_with_provider!(get_claude, ...)`
- `:869-914` — `openai` **inlined** because `get_openai(session.id)` takes the session id
- `:915` — `gemini`
- `:916` — `zai`
- `:917` — `codex`
- `:944-952` — `github-copilot | copilot` (PROV-057 Layer 3)

The OpenAI inlined arm at `:877-895` contains the same 5 calls as the macro (verified manually):
gather wrappers `:877`, create_rig_agent `:879`, add_tool loop `:887`, set_mcp_tool_server_handle `:892`, with_default_depth `:896`.

### 3. Custom-provider fallthrough — `codelet/agent-loop/src/agent_loop.rs:953-1020`

The `_` arm at `:953` dispatches to `CustomProvider::create_rig_agent` (`:966-973`) and then performs the same 4 follow-up calls: gather `:981`, add_tool loop `:985`, set_mcp_tool_server_handle `:991`, with_default_depth `:995`.

### 4. Per-provider `create_rig_agent` signature

All 7 providers expose `create_rig_agent(session_id, preamble, thinking) -> Agent<Model>` or equivalent handle (compile-time tested in RPC-082's `rpc082_role_injection.rs`).

The rig `Agent` is built with the full tool slate inside each provider's `create_rig_agent`. E.g. `codelet/providers/src/claude.rs:507-595` registers Read / Write / Edit / Bash / Grep / Glob / MultiEdit / NotebookRead / NotebookEdit / WebFetch / WebSearch / Task / TodoWrite / Plan / AskFollowupQuestion / AttemptCompletion etc.

### 5. `RigAgent::with_default_depth` — `codelet/core/src/rig_agent.rs:47`

`pub fn with_default_depth(agent: Agent<M>) -> Self` — wraps the rig agent in the tool-use loop with the default depth cap. Returns `RigAgent<M>`.

### 6. `gather_mcp_tool_wrappers` — `codelet/tools/src/mcp.rs:1041`

`pub fn gather_mcp_tool_wrappers(session_id: uuid::Uuid) -> Vec<McpToolWrapper>` — turn-start collection (non-blocking try_read).

## Conclusion

All five canonical calls are present in all three dispatch paths:
- macro body (5/5 providers go through it)
- OpenAI inlined arm (verbatim repeats 5/5)
- CustomProvider fallthrough (verbatim repeats 4/5; create_rig_agent is the custom variant)

**This is coverage-only work.** Tests must:

1. **Structural** — source-string assertions on each of the 3 dispatch paths to lock in the 5 calls.
2. **Behavioural** — a stub-provider end-to-end round-trip that proves a ToolCall + ToolResult flow through the agent_loop and reaches the BackgroundOutput sink.
3. **Compile-time** — closure typing for `with_default_depth(agent)` and `gather_mcp_tool_wrappers(session.id)` signatures to prevent silent drift.
