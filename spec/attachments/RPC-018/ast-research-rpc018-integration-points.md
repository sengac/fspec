# RPC-018 — AST research

This is the AST-driven research phase mandated by ACDD discovery before
moving RPC-018 to `testing`. We map each integration seam touched by
RPC-018 to the precise file:line where the existing code lives so the
test-writing phase can pin assertions against real symbols.

## Existing symbols touched by RPC-018

| Symbol | File:Line | Touch type |
|---|---|---|
| `pub fn render_with_store` on AgentView | codelet/fspec-tui/src/views/agent.rs:181 | EXTEND — gains Header(1) + Footer(1) rows around the existing Scrollback + Input |
| `pub struct AgentViewStore` | codelet/fspec-tui/src/store/agent_view.rs:20 | EXTEND — gains 4 new fields (model_info_by_session, token_state_by_session, thinking_level_by_session, workspace) |
| `pub enum StreamChunk` (TokenUpdate / ContextFillUpdate variants) | codelet/rpc-types/src/lib.rs:420 | READ — record_chunk derives TokenState from these two variants |
| `pub trait FspecService` | codelet/rpc/src/lib.rs:51 | EXTEND — gains 3 new RPC methods |
| `pub trait FspecBackend: Send + Sync` | codelet/fspec-tui/src/transport/mod.rs:57 | EXTEND — gains 3 new methods + impl in embedded.rs + websocket.rs |
| `pub fn get_current_branch` | codelet/git/src/status.rs:165 | CALL — get_workspace_info uses this helper |
| `pub enum Action` | codelet/fspec-tui/src/components/mod.rs:86 | EXTEND — gains ModelInfoLoaded / ThinkingLevelLoaded / WorkspaceInfoLoaded variants |
| `pub trait SessionManagerHandle` | codelet/core/src/session_manager_handle.rs:53 | EXTEND — gains get_model_info + get_thinking_level with default impls |

## Patterns followed from RPC-015 / RPC-017

* `SharedFspecService.with_cwd(PathBuf) -> Self` builder pattern
  (codelet/rpc/src/lib.rs:217). RPC-018 `get_workspace_info` consumes
  this same `cwd` slot.
* Shared types under `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`
  (codelet/rpc-types/src/lib.rs:72 — see `CheckpointCounts`). RPC-018's
  `ModelInfo` and `WorkspaceInfo` adopt the same gate; `ThinkingLevel`
  uses `#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]`
  per `SessionStatus` (codelet/rpc-types/src/lib.rs:132).
* Cross-transport parity tests pattern: see
  `codelet/fspec-tui/tests/checkpoint_counts_rpc015.rs` and
  `codelet/fspec-tui/tests/move_work_unit_rpc017.rs`.
* Source-shape tests pattern: see
  `codelet/fspec-tui/tests/source_shape_rpc015.rs` and
  `codelet/fspec-tui/tests/source_shape_rpc017.rs`.
* BoardView header strip orchestrator pattern: see
  `codelet/fspec-tui/src/views/board/header.rs` (RPC-015) — composes
  sub-widgets without inlining their bodies.

## StreamChunk variants relevant to TokenState derivation

```rust
StreamChunk::TokenUpdate {
    tokens: TokenTracker,  // contains input_tokens + output_tokens
}
StreamChunk::ContextFillUpdate {
    context_fill: ContextFillInfo,  // contains fill_percentage
}
```

All other variants leave `TokenState` unchanged.

## Open implementation choices

1. Whether AgentView's record_chunk gets a `&mut TokenState` parameter
   or whether App::dispatch separately mutates the by-session map.
   Decision: have `App::dispatch` on `Action::ChunkReceived` mutate the
   AgentViewStore.token_state_by_session map directly (parallel to how
   it already updates scrollback through `navigator.agent.record_chunk`).
   This keeps `AgentView` UI-only.
2. Whether SessionManagerHandle.get_model_info/get_thinking_level
   defaults live in the trait or in a base impl. Decision: in the
   trait via `fn ... { ... }` default-body syntax — matches the
   pattern used elsewhere in this crate.
3. Where the `home::home_dir` shortening lives. Decision: inside
   `views/agent/footer.rs::render` so the widget is the only place
   that knows about `~` substitution — keeps WorkspaceInfo cross-OS
   neutral.

## Files to be created

| File | Purpose | LoC budget |
|---|---|---|
| codelet/fspec-tui/src/views/agent/mod.rs | Replaces `views/agent.rs`; AgentView struct + render_with_store layout + handle_event | <300 |
| codelet/fspec-tui/src/views/agent/header.rs | `SessionHeader<'a>` widget | <200 |
| codelet/fspec-tui/src/views/agent/footer.rs | `SessionFooter<'a>` widget | <200 |
| codelet/fspec-tui/tests/view_agent_unit_rpc018.rs | Header + Footer + layout render tests | <300 |
| codelet/fspec-tui/tests/agent_chrome_parity_rpc018.rs | Cross-transport parity for the 3 new RPCs | <300 |
| codelet/fspec-tui/tests/app_bootstrap_rpc018.rs | bootstrap + dispatch wiring tests | <300 |
| codelet/fspec-tui/tests/source_shape_rpc018.rs | source-shape regression | <300 |

## NAPI surface delta (additive only)

| New export | File | Delegates to |
|---|---|---|
| `napi::get_workspace_info(cwd)` | codelet/napi/src/git.rs | `codelet_git::status::get_current_branch` |
| `napi::get_model_info(sessionId)` | codelet/napi/src/session_manager.rs | `SessionManagerHandle::get_model_info` (default impl in RPC-018) |
