# RPC-035 — Reduce `codelet-napi` persistence to thin `napi_bindings.rs` shims

**Parent:** RPC-030 · **Phase:** 1.5 · **Estimate:** 3 pts · **Depends on:** RPC-034

## Goal

After RPC-031..RPC-034, all pure-Rust persistence logic lives in `codelet-core::persistence`. This card cleans up `codelet/napi/src/persistence/` so it contains ONLY:

- `mod.rs` — re-export facade
- `napi_bindings.rs` — `#[napi]` thin shims (the only remaining purpose of the module)
- Per-NAPI wire structs (`NapiSessionManifest`, `NapiStoredMessage`, etc.) if they cannot live in `codelet-rpc-types`
- Test helpers (`tests.rs`, `lazy_init_tests.rs`) — keep if they exercise the NAPI-bridge layer specifically; otherwise move to `codelet-core::persistence::tests`.

## Current state of `napi_bindings.rs`

`codelet/napi/src/persistence/napi_bindings.rs` — 35,299 bytes, 939 lines. Every export is a `#[napi]` `pub fn` calling free functions in `super::*` (which currently live in `persistence/mod.rs`).

Key exports (66 functions):

- **Sessions:** `persistence_set_data_directory`, `persistence_get_data_directory`, `persistence_create_session`, `persistence_create_session_with_provider`, `persistence_load_session`, `persistence_resume_last_session`, `persistence_list_sessions`, `persistence_delete_session`, `persistence_rename_session`, `persistence_fork_session`, `persistence_merge_messages`, `persistence_cherry_pick`
- **Messages:** `persistence_append_message`, `persistence_append_message_with_metadata`, `persistence_get_message`, `persistence_get_session_messages`, `persistence_get_session_messages_full`
- **Blobs:** `persistence_store_blob`, `persistence_get_blob`, `persistence_blob_exists`
- **History:** `persistence_add_history`, `persistence_get_history`, `persistence_search_history`
- **Tokens/compaction:** `persistence_update_session_tokens`, `persistence_set_session_tokens`, `persistence_set_compaction_state`, `persistence_clear_compaction_state`, `persistence_cleanup_orphaned_messages`
- **Envelope (JSON-string boundary):** `persistence_store_message_envelope`, `persistence_get_message_envelope`, `_raw`, `_get_session_message_envelopes`, `_full`, `_raw`, `_raw_full`

Plus NAPI wire structs with `From<…>` impls: `NapiSessionManifest`, `NapiForkPoint`, `NapiMergeRecord`, `NapiCompactionState`, `NapiTokenUsage`, `NapiStoredMessage`, `NapiHistoryEntry`, `NapiAppendResult`, `NapiCherryPickResult`.

## Work to do

### Step 1 — Update imports in `napi_bindings.rs`

Every `super::function_name(...)` call becomes `codelet_core::persistence::function_name(...)`. Or simpler: at the top of `napi_bindings.rs` add `use codelet_core::persistence::{...};` so the bodies stay short.

### Step 2 — Delete dead files

After RPC-031..RPC-034:
- `persistence/message_envelope.rs` — 1-line re-export shim → delete (covered by `mod.rs` re-export).
- `persistence/storage.rs` — delete (both stores moved).
- `persistence/types.rs` — delete (types moved).
- `persistence/blob.rs` — delete.
- `persistence/blob_processing.rs` — delete.
- `persistence/message_index.rs` — delete.
- `persistence/history.rs` — delete (already a re-export shim).

### Step 3 — Reduce `persistence/mod.rs`

Final shape:

```rust
//! NAPI bindings for codelet_core::persistence.
pub use codelet_core::persistence::*;

#[cfg(not(feature = "noop"))]
mod napi_bindings;
#[cfg(not(feature = "noop"))]
pub use napi_bindings::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod lazy_init_tests;
```

### Step 4 — Decide where tests go

`persistence/tests.rs` (89,926 bytes, 48 tests) and `lazy_init_tests.rs` (14,818 bytes, 9 tests) test the global-singleton free-function layer, not the NAPI bridge specifically. **Move them** to `codelet/core/src/persistence/tests.rs` and `lazy_init_tests.rs`.

NAPI-bridge-specific tests (round-tripping `Napi*` wire structs) stay in `codelet/napi/tests/session_persistence_test.rs` and `subordinate_session_persistence_test.rs`.

### Step 5 — Keep NAPI wire structs co-located with bindings

`NapiSessionManifest`, `NapiStoredMessage`, etc. all live inside `napi_bindings.rs` and stay there. They are NAPI-bridge-only concerns. Their `From<SessionManifest>` impls use the types from `codelet_core::persistence` (no change needed beyond the import path).

## Acceptance criteria

1. `codelet/napi/src/persistence/` contains exactly: `mod.rs`, `napi_bindings.rs`, `tests.rs` (only if kept), `lazy_init_tests.rs` (only if kept). Nothing else.
2. `codelet/napi/index.d.ts` is byte-identical to before this card (TS-facing API surface unchanged).
3. `cargo build -p codelet-napi --features napi` and `--features noop` both pass.
4. All 80+ persistence tests still pass.
5. Snapshot `codelet/napi/index.d.ts` before + after this card and assert byte-identical.

## Exit criterion (Phase 1 complete)

After this card: `codelet/napi/src/persistence/` is a pure adapter. All persistence logic, types, and singletons live in `codelet-core::persistence`. Both frontends can use persistence without depending on `napi`.

## Risks

- `index.d.ts` regeneration in `napi-rs` is order-sensitive. If `cargo build` reorders any field, the TS frontend breaks. Lock the field order in `Napi*` wire structs explicitly.
- The `noop` feature gate disables NAPI bindings entirely. Keep `#[cfg(not(feature = "noop"))]` on every `#[napi]` function.

## Out of scope

- Anything outside `codelet/napi/src/persistence/`.
- Phase 2 (rpc-types widening) → RPC-036.
