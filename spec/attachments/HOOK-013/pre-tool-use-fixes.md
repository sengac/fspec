# HOOK-013: pre_tool_use Hook Integration — Complete Wiring Report

**Date:** 2026-03-20
**Status:** All tool calls wired ✅

## Summary

pre_tool_use lifecycle hooks are now wired into **every** tool's `call()` method
that can be invoked by the agent. The check fires as the first operation in each
`call()`, before any side effects.

## Hook Wiring — Full Tool Inventory (22 `impl Tool for`)

### Facade Wrappers (9) — all use `check_pre_tool_hook()`

| Tool | Location | Status |
|------|----------|--------|
| FacadeToolWrapper | wrapper.rs call() | ✅ |
| HitlToolFacadeWrapper | wrapper.rs call() | ✅ |
| FileToolFacadeWrapper | wrapper.rs call() | ✅ |
| FspecToolFacadeWrapper | wrapper.rs call() | ✅ |
| BashToolFacadeWrapper | wrapper.rs call() | ✅ |
| SearchToolFacadeWrapper | wrapper.rs call() | ✅ |
| LsToolFacadeWrapper | wrapper.rs call() | ✅ |
| BridgeToolFacadeWrapper | wrapper.rs call() | ✅ |
| ExecToolFacadeWrapper | wrapper.rs call() | ✅ |

### Standalone Tools (10) — all use `pre_tool_hook_check()`

| Tool | Location | Status |
|------|----------|--------|
| AgentManagerTool | agent_manager/mod.rs call() | ✅ |
| SessionSearchTool | session_search/mod.rs call() | ✅ |
| DeepSearchTool | deep_search/mod.rs call() | ✅ |
| ScheduleTool | schedule/mod.rs call() | ✅ |
| GraphSearchTool | graph_search/mod.rs call() | ✅ |
| InjectSummaryTool | inject_summary.rs call() | ✅ |
| RequestUserInputTool | request_user_input.rs call() | ✅ |
| WebSearchTool | web_search.rs call() | ✅ |
| McpToolWrapper | mcp.rs call() | ✅ |
| ConnectMcpTool | mcp.rs call() | ✅ |

### Stub/Proxy Tools (3) — N/A (never execute real logic)

| Tool | Reason |
|------|--------|
| FspecTool | Always returns Err — facade wrapper handles actual calls |
| BridgeTool | Always returns Err — facade wrapper handles actual calls |
| UnifiedExecTool | Never registered directly — ExecToolFacadeWrapper used instead |

## Session Manager Integration

| Integration Point | Location | Status |
|-------------------|----------|--------|
| register_pre_tool_hook (normal session) | session_manager.rs:3405 | ✅ |
| register_pre_tool_hook (isolated session) | session_manager.rs:3640 | ✅ |
| unregister_pre_tool_hook (destroy) | session_manager.rs:3851 | ✅ |
| session_start hooks fired | agent_loop start | ✅ |
| session_end hooks fired | agent_loop channel close | ✅ |
| user_prompt_submit hooks fired | agent_loop before processing | ✅ |
| post_tool_use hooks fired | agent_loop ToolResult event | ✅ |

## Code Smells Fixed

1. **`response.rs` — removed dead `suppress_output` field**: Deserialized but never read. YAGNI.
2. **`grep.rs` — removed dead `is_context` field on `MatchLine`**: Set but never read.
3. **`pre_tool_hook.rs` — eliminated `unreachable!()` in Deny match**: Replaced convoluted double-match with direct pattern destructuring.
4. **`git.rs` / `napi_bindings.rs` — removed spurious `#[allow(dead_code)]` on NAPI functions**: `#[napi]` macro handles FFI export; the suppress was unnecessary.
5. **`web_search.rs` — restored session_id field**: Was incorrectly removed. TOOL-014 requires ALL tools to take session_id. The field is now actively used for pre_tool_hook_check.
6. **`mcp.rs` — added pre_tool_hook_check to McpToolWrapper and ConnectMcpTool**: Both were missing hook checks.
7. **`web_search.rs` — added pre_tool_hook_check to WebSearchTool**: Was missing hook check.

## Build Verification

- `cargo check`: ✅ zero errors
- `cargo clippy -- -D warnings`: ✅ zero warnings
- `cargo test -p codelet-tools --lib -- pre_tool_hook`: ✅ 6/6 pass
