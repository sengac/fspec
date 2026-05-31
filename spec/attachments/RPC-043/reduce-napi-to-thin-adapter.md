# RPC-043 — Reduce `codelet-napi` to thin adapter (`session_bindings.rs`); update `Cargo.toml`

**Parent:** RPC-030 · **Phase:** 4.6-4.7 · **Estimate:** 5 pts · **Depends on:** RPC-042

## Goal

Delete `codelet/napi/src/session_manager.rs` (the file shrank to mostly `#[napi]` wrappers after RPC-039/040). Replace with `codelet/napi/src/session_bindings.rs` (~500 LOC target) that:

1. Holds a global `Arc<codelet_sessions::SessionManager>` singleton.
2. Subscribes once at startup to `chunks_tx` (broadcast) and fans into the JS `ThreadsafeFunction`.
3. Wraps every public `SessionManager` / `BackgroundSession` method behind a `#[napi]` function doing ONLY type conversion (Rust ↔ NAPI bridge types).

`codelet/napi/index.d.ts` must remain byte-identical so the TS frontend sees no change.

## Files to delete

- `codelet/napi/src/session_manager.rs` (after extracting any remaining helpers).

## File to create

`codelet/napi/src/session_bindings.rs`. Target structure:

```rust
//! Thin NAPI adapter over `codelet_sessions::SessionManager`.
//!
//! This module exists only to bridge the Rust agent loop to the TypeScript
//! frontend via `napi-rs`. Every function below performs type conversion
//! and delegates to the real implementation in `codelet-sessions`.

use codelet_sessions::{SessionManager, BackgroundSession};
use codelet_rpc_types::*;

static SESSION_MANAGER: OnceCell<Arc<SessionManager>> = OnceCell::new();

fn manager() -> &'static Arc<SessionManager> {
    SESSION_MANAGER.get_or_init(|| Arc::new(SessionManager::new()))
}

#[napi]
pub fn session_manager_create(...) -> Result<String> { /* type-convert + call */ }

#[napi]
pub fn session_manager_create_with_id(...) -> Result<()> { /* type-convert + call */ }

#[napi]
pub fn session_manager_create_isolated(...) -> Result<NapiIsolatedSessionResult> {
    let result = tokio_handle.block_on(
        manager().create_isolated_session_with_id(...)
    )?;
    Ok(result.into())
}

// ... 65+ more #[napi] wrappers
```

## All wrappers to preserve

From the audit of `codelet/napi/src/session_manager.rs`:

| Original line | `#[napi]` function |
|---|---|
| 6240 | `session_manager_create` |
| 6253 | `session_manager_create_with_id` |
| 6288 | `session_manager_create_isolated` |
| 6301 | `session_manager_list` |
| 6307 | `session_manager_destroy` |
| 6331 | `session_set_global_chunk_callback` ← rewritten in RPC-041 |
| 6543 | `session_set_active` |
| 6555 | `session_send_input` |
| 6562 | `session_interrupt` |
| 6577 | `session_clear_history` |
| 6585 | `session_get_status` |
| 6596 | `session_get_compaction_progress` |
| 6612 | `session_get_pause_state` |
| 6622 | `session_get_hitl_request` |
| 6644 | `session_pause_resume` |
| 6655 | `session_pause_confirm` |
| 6671 | `session_pause_triple` |
| 6701 | `session_send_fspec_result` |
| 6724 | `session_send_hitl_response` |
| 6757 | `session_get_base_thinking_level` |
| 6768 | `session_set_base_thinking_level` |
| 6780 | `session_get_next` |
| 6787 | `session_get_prev` |
| 6794 | `session_get_first` |
| 6801 | `session_clear_active` |
| 6812 | `session_get_turn_details` |
| 6884 | `session_set_model` (async) |
| 7012 | `session_set_model_profile` (async) |
| 7144 | `session_get_model` |
| 7170 | `session_get_internal_provider` |
| 7197 | `session_get_tokens` |
| 7209 | `session_get_debug_enabled` |
| 7216 | `session_set_debug_enabled` |
| 7227 | `session_get_pending_input` |
| 7237 | `session_set_pending_input` |
| 7245 | `session_get_buffered_output` |
| 7262 | `session_set_role` |
| 7280 | `session_get_role` |
| 7295 | `session_is_scheduled` |
| 7302 | `session_schedule_name` |
| 7319 | `loop_register` |
| 7382 | `loop_cancel` |
| 7390 | `loop_list` |
| 7420 | `session_get_subordinate` |
| 7433 | `session_get_supervisors` |
| 7454 | `session_set_observed_correlation_ids` |
| 7465 | `session_clear_observed_correlation_ids` |
| 7474 | `session_get_merged_output` |
| 7519 | `session_restore_messages` |
| 7692 | `session_restore_token_state` |
| 7727 | `toggle_debug` |
| 7741 | `session_update_debug_metadata` |
| 7772 | `session_toggle_debug` |
| 7874 | `session_compact` |
| 7975 | `test_provider_connection` |
| 8007 | `session_set_work_unit_context` |
| 8022 | `session_get_work_unit_context` |
| 8043 | `session_get_active` |
| 8084 | `session_validate_path` |
| 8385 | `session_get_effective_cwd` |
| 8399 | `session_is_isolated` |
| 8431 | `session_execute_bash` |
| 8574 | `list_providers` |
| 8583 | `show_provider` |
| 8593 | `validate_provider` |
| 8601 | `test_provider` (async) |
| 8616 | `init_provider` |
| 8642 | `get_model_info` |

Total: ~66 `#[napi]` wrappers. Each becomes a ~5-10 LOC adapter.

## `codelet/napi/Cargo.toml` changes

Add:
```toml
codelet-sessions = { path = "../sessions" }
```

Remove (now reached transitively through `codelet-sessions`):
- `codelet-cli` (only kept if NAPI has other direct callers — audit `lib.rs`)
- `codelet-providers`
- `codelet-git`
- `codelet-tools`

**Keep:**
- `codelet-core` (persistence types, lifecycle hooks)
- `codelet-rpc-types` (wire types)
- `codelet-common` (debug capture)

Audit `codelet/napi/src/lib.rs` and grep for `codelet_cli::`, `codelet_providers::`, etc. — remove dependencies only after confirming no direct uses remain.

## `codelet/napi/src/lib.rs` update

```rust
pub mod persistence;        // unchanged (RPC-035)
pub mod session_bindings;   // NEW (this card)
// pub mod session_manager; // DELETED

pub use session_bindings::*;
```

## NAPI wire structs

Keep `NapiSessionManifest`, `NapiStoredMessage`, etc. inside `session_bindings.rs` (or factor into `napi_types.rs`). These have `From<...>` impls bridging `codelet_core` / `codelet_rpc_types` shapes into `#[napi(object)]`-decorated shapes.

## Acceptance criteria

1. `codelet/napi/src/session_manager.rs` no longer exists.
2. `codelet/napi/src/session_bindings.rs` exists with all 66 `#[napi]` wrappers.
3. Each wrapper is ≤ 15 LOC (delegates to `codelet_sessions`).
4. `codelet/napi/Cargo.toml` no longer lists `codelet-cli`, `codelet-providers`, `codelet-git`, `codelet-tools` as direct dependencies (only reached transitively).
5. `cargo build -p codelet-napi` passes.
6. `cargo build -p codelet-napi --features noop` passes.
7. `codelet/napi/index.d.ts` regenerated → diff against pre-RPC-039 baseline is **EMPTY** (no removals, no renames, no new fields).
8. `cargo test -p codelet-napi` passes.
9. Boot TS frontend end-to-end → assert every TS-level smoke test passes (covered in RPC-068).

## Risks

- `index.d.ts` is auto-generated by `napi-rs build`. Subtle field-order changes between releases of `napi-derive` can cause spurious diffs. Pin the `napi-derive` version.
- File size: `session_bindings.rs` may exceed the 300-line file convention. **Document why this exception is acceptable** (it's a pure adapter layer; splitting fragments the audit surface). Alternative: split into `session_bindings/persistence.rs`, `session_bindings/sessions.rs`, etc.
- `tokio::runtime::Handle::current()` inside `#[napi]` async functions: napi-rs sets up its own runtime. Confirm `block_on` is safe in this context (it usually is).

## Out of scope

- Wiring `fspec` binary → RPC-044.
- AgentView wiring → RPC-045+.
