# RPC-062 — MCP injection plumbing in extracted `SessionManager`

**Parent:** RPC-030 · **Phase:** 7.9 · **Estimate:** 3 pts · **Depends on:** RPC-061

## Goal

Confirm MCP (Model Context Protocol) injection plumbing works correctly inside the extracted `codelet-sessions::SessionManager`. The actual code moved with the lift (RPC-039/040), but this card audits the wire-up + adds tests.

## Source

`McpInjection` type already lives in `codelet-tools` (NAPI-free, imported at `session_manager.rs:41`):

```rust
use codelet_tools::McpInjection;
```

Wire-up sites:
- `SessionManager::create_session_with_id` (line ~3499 of original) calls `codelet_tools::init_mcp_session(session_id, ...)`.
- `SessionManager::create_isolated_session_with_id` (line ~3760) also calls `init_mcp_session`.
- `SessionManager::destroy_session` calls `codelet_tools::cleanup_mcp_session` (line ~3971).
- The agent loop in `BackgroundSession` consumes `mcp_injection_rx` (audit `codelet_cli::interactive::run_agent_stream_with_images` for the receiver).

## What to verify in this card

1. `init_mcp_session` is reached inside `codelet-sessions` (not stubbed out by the move).
2. `cleanup_mcp_session` is reached on destroy.
3. The `mcp_injection_rx` receiver in the agent loop is fed by `codelet_tools::register_mcp_handler(...)` or equivalent.
4. MCP tools surface in the session's available tool list.

No new RPC surface is needed — MCP injection is purely internal to the agent loop.

## Tests to add

`codelet/sessions/tests/mcp_injection.rs`:

```rust
#[tokio::test]
async fn mcp_session_initialised_on_create() {
    // 1. Start a fake MCP server on a local TCP port.
    // 2. Configure ~/.fspec/mcp_servers.json to point at it.
    // 3. SessionManager::new() + create_session.
    // 4. Send an input that would invoke an MCP tool.
    // 5. Assert the MCP server received the call.
}

#[tokio::test]
async fn mcp_cleaned_up_on_destroy() {
    // Same setup, then destroy_session.
    // Assert the MCP server received a "session ended" notification.
}
```

If a real MCP server is too heavy, use a tokio-channel-backed stub registered via `register_pre_tool_hook`.

## Acceptance criteria

1. `codelet-sessions::SessionManager::create_session_with_id` reaches `init_mcp_session`.
2. `codelet-sessions::SessionManager::destroy_session` reaches `cleanup_mcp_session`.
3. New integration tests in `codelet/sessions/tests/mcp_injection.rs` pass.
4. TS-side MCP smoke test (running the TS frontend against a session + invoking an MCP tool) continues to work (covered transitively by RPC-068).

## Risks

- `init_mcp_session` may currently rely on NAPI-thread-locals or `tokio::spawn` semantics that changed when moved to a new crate. Verify.
- MCP-handler registration is global (per-process). If the lift duplicates registration on multiple SessionManager constructions, that's a bug — confirm singleton semantics.

## Out of scope

- New RPC methods for MCP (none needed — agent loop only).
- Adding new MCP transport types.
