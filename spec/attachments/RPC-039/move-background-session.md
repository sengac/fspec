# RPC-039 — Move `BackgroundSession` from `codelet-napi` into `codelet-sessions`, replace NAPI references

**Parent:** RPC-030 · **Phase:** 4.2 · **Estimate:** 8 pts · **Depends on:** RPC-038

## Goal

Move the entire `BackgroundSession` struct + impl out of `codelet/napi/src/session_manager.rs` (lines 459–1362) into `codelet/sessions/src/background_session.rs`. Replace every `napi::` reference, every `crate::persistence::` import, and every `crate::types::` import with the lifted equivalents.

## Source — `codelet/napi/src/session_manager.rs` lines 459–1362

### Fields (lines 459–600) — ~30 fields

Key fields that need adapter changes:

| Line | Field | Type | Change |
|---|---|---|---|
| 484 | `inner` | `Arc<Mutex<codelet_cli::session::Session>>` | no change |
| 490 | `input_tx` | `mpsc::Sender<PromptInput>` | no change |
| 507 | `debug_capture` | `Arc<PoisonRecoveryMutex<DebugCaptureManager>>` | no change (codelet_common, NAPI-free) |
| 513 | `supervisor_broadcast` | `broadcast::Sender<StreamChunk>` | use `codelet_rpc_types::StreamChunk` |
| 539 | `pause_state` | `RwLock<Option<PauseState>>` | now uses `codelet_rpc_types::PauseState` (mirrors `codelet_tools::tool_pause::PauseState`) |
| 546-547 | `fspec_response_tx/rx` | uses `crate::types::FspecResult` | replace with `codelet_rpc_types::FspecResult` |
| 550-551 | `hitl_response_tx/rx` | `codelet_tools::request_user_input::HitlResponse` | no change (codelet-tools is NAPI-free) |
| 555 | `hitl_request` | `codelet_tools::request_user_input::HitlRequest` | no change |
| 567 | `work_unit_context` | `RwLock<Option<WorkUnitContext>>` | now uses `codelet_rpc_types::WorkUnitContext` |
| 587 | `pending_dag_content` | `Arc<std::sync::Mutex<Option<String>>>` | no change |
| 599 | `lifecycle_hooks` | `Option<Arc<CompiledLifecycleHooks>>` | use `codelet_core::lifecycle_hooks::CompiledLifecycleHooks` |

### Methods (lines 602–1362) — ~50 methods

All methods move verbatim. Specific replacements:

- **Line ~1247 in `send_input`**: `napi::Error::from_reason(format!("..."))` → `String` errors. Change the return type from `napi::Result<()>` to `Result<(), String>`.
- **Imports of `crate::persistence::{...}`**: every usage of `load_session, append_message_with_metadata, update_session_tokens, MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent` becomes `codelet_core::persistence::{...}` (these moved in RPC-031..RPC-035).
- **Imports of `crate::types::{...}`**: replace with `codelet_rpc_types::{...}` (RPC-036). Notably `FspecResult`, `WorkUnitContext`, `PauseState`, `HitlRequest`, `HitlResponse`.

### Key method bodies that need careful adaptation

**`handle_output` (line 931–966):**
- Currently calls `GLOBAL_CHUNK_CALLBACK.call(session_id, chunk)`.
- In this card, replace with a call to `self.chunks_tx.send((session_id, chunk))` where `chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>` is a new field added to `BackgroundSession` (or borrowed from the parent `SessionManager`).
- The actual deletion of `GLOBAL_CHUNK_CALLBACK` happens in RPC-041 — for this card, leave it dead-code-tolerated by NOT removing it yet (or wire `chunks_tx` to be filled in by RPC-041). Mark with a TODO comment.

**`send_input` (line 1237–1259):**
- Buffers `user_input` chunk via `self.output_buffer.write().push(...)`.
- Sends `PromptInput` on `input_tx`.
- Replace `napi::Error::from_reason` with `String`.
- The thinking_config parameter stays `Option<String>` (JSON-encoded) — no change.

**`get_info` (line 1327):**
- Returns `SessionInfo` (already in `codelet_rpc_types`). No change.

## Imports to add at top of `background_session.rs`

```rust
use std::sync::Arc;
use std::sync::atomic::*;
use std::path::PathBuf;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc, Notify};
use uuid::Uuid;

// Lifted persistence (Phase 1)
use codelet_core::persistence::{
    load_session, append_message_with_metadata, update_session_tokens,
    MessageEnvelope, MessagePayload, UserMessage, UserContent,
    AssistantMessage, AssistantContent,
};

// Wire types (Phase 2)
use codelet_rpc_types::{
    StreamChunk, SessionStatus, SessionId, SessionInfo,
    PauseState, PauseResponse, FspecResult, WorkUnitContext,
    CompactionProgress, ThinkingConfig,
};

// Internal subsystem deps (all NAPI-free already)
use codelet_cli::session::Session;
use codelet_cli::interactive_helpers::*;
use codelet_cli::session::context_gathering::*;
use codelet_cli::compaction_threshold::*;
use codelet_common::debug_capture::*;
use codelet_core::lifecycle_hooks::*;
use codelet_git::ghost_commit::*;
use codelet_tools::{McpInjection, request_user_input::*, tool_pause::*};
```

## NAPI-side cleanup (in this card)

Delete lines 459–1362 from `codelet/napi/src/session_manager.rs`. Replace with:

```rust
pub use codelet_sessions::background_session::BackgroundSession;
```

(`codelet-napi/Cargo.toml` already added `codelet-sessions` via RPC-038.)

The rest of `session_manager.rs` (lines 1–458, 1363–8645) stays in NAPI for this card; the `SessionManager` struct + impl moves in RPC-040.

## Acceptance criteria

1. `codelet/sessions/src/background_session.rs` contains the full `BackgroundSession` struct + impl.
2. No `napi::` references inside `background_session.rs`.
3. `crate::persistence` → `codelet_core::persistence` (no more `crate::persistence` imports in moved code).
4. `crate::types::FspecResult` / `WorkUnitContext` etc. → `codelet_rpc_types::...`.
5. `cargo build -p codelet-sessions` passes.
6. `cargo build -p codelet-napi` passes (re-export keeps consumers working).
7. NAPI tests in `codelet/napi/tests/` still pass.
8. No new behavioural changes — purely a code move with import-path rewrites.

## Risks

- `BackgroundSession::handle_output` is the hottest path in the agent loop. Make sure the temporary "broadcast not yet wired" state of this card doesn't drop chunks. Mitigation: keep the `GLOBAL_CHUNK_CALLBACK.call(...)` call alongside the new (no-op) `chunks_tx.send(...)` until RPC-041 wires both ends.
- Method count is ~50. Move them in a single commit (no partial moves) to keep the file in a compilable state.
- `pause_state` field uses `codelet_tools::tool_pause::PauseState`. `codelet_rpc_types::PauseState` is a mirror — choose ONE source of truth. Recommend: `BackgroundSession` continues to use the `codelet_tools` type internally; the wire boundary uses `codelet_rpc_types`. Conversion impl on both types.

## Out of scope

- Moving `SessionManager` → RPC-040.
- Removing `GLOBAL_CHUNK_CALLBACK` → RPC-041.
- Implementing `SessionManagerHandle` on the new types → RPC-042.
