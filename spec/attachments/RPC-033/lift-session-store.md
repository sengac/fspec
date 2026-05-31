# RPC-033 — Lift `SessionStore` manifest + `load_session` + `append_message_with_metadata` into `codelet-core::persistence::manifest`

**Parent:** RPC-030 · **Phase:** 1.3 · **Estimate:** 5 pts · **Depends on:** RPC-032

## Goal

Move the on-disk session-manifest layer (the `SessionStore` struct, the `MESSAGE_STORE`/`SESSION_STORE`/`HISTORY_STORE` global facade, and all of the free functions in `codelet/napi/src/persistence/mod.rs`) into `codelet-core::persistence::manifest`. After this card, the entire persistence stack (envelopes + messages + sessions + blobs) is owned by `codelet-core`.

## Source locations (move from)

### `codelet/napi/src/persistence/storage.rs` lines 352-620 — `SessionStore`

```rust
pub struct SessionStore { /* in-memory cache + cwd */ }
impl SessionStore {
    pub fn new(), create, create_with_provider, save,
    pub fn get(&Uuid) -> Option<&SessionManifest>, get_mut,
    pub fn load, get_last_session, resume_last,
    pub fn list_for_project, list_all,
    pub fn delete, rename, fork,
}
```

### `codelet/napi/src/persistence/mod.rs` — 24,664 bytes, 780 lines (the free-function facade)

All of the following move:

| Function | Signature |
|---|---|
| `set_data_directory` | `(PathBuf) -> Result<(), String>` |
| `get_data_dir` | `() -> Result<PathBuf, String>` |
| `ensure_directories` | `() -> Result<(), String>` |
| `create_session` / `create_session_with_provider` | construct a `SessionManifest` |
| `save_session` | `(&SessionManifest) -> Result<(), String>` |
| `load_session` | `(Uuid) -> Result<SessionManifest, String>` |
| `resume_last_session`, `fork_session`, `merge_messages`, `cherry_pick` | |
| `list_sessions` (pub(crate)), `switch_session`, `delete_session`, `rename_session` | |
| `append_message`, `append_message_with_metadata`, `update_message_metadata` | |
| `cleanup_orphaned_messages` | |
| `get_message`, `get_session_messages`, `get_session_messages_full` | |
| `update_session_tokens`, `set_session_tokens`, `set_compaction_state`, `clear_compaction_state` | |
| `get_session_lineage` (+ `pub struct SessionLineage`) | |
| `list_sessions_for_project`, `list_all_sessions` | |

### `codelet/napi/src/persistence/types.rs` (292 lines)

`TokenUsage`, `MergeRecord`, `PastedContent`, `MessageSource`, `MessageRef`, `ForkPoint`, `CompactionState`, **`SessionManifest`** (and its `new`/`with_provider`/`add_message`/`record_merge`/`message_count`/`update_token_usage` methods).

Already partially lifted: line 12 of this file has `pub use codelet_core::persistence::HistoryEntry`.

### `codelet/napi/src/persistence/history.rs` (15 lines) — already a re-export shim

```rust
pub use codelet_core::persistence::history::HistoryStore;
pub use codelet_core::persistence::HistoryEntry;
```

## Target location

`codelet/core/src/persistence/manifest.rs` (struct + impl) + `codelet/core/src/persistence/mod.rs` (free-function facade with the global `lazy_static! Mutex<Option<...>>` singletons).

Add to `codelet/core/src/persistence/mod.rs`:
```rust
pub mod manifest;
pub use manifest::*;
// existing: pub mod message_envelope; pub mod messages; pub mod blob; pub mod history;
```

## NAPI re-export shim

After this card, `codelet/napi/src/persistence/mod.rs` becomes ~30 lines:

```rust
//! All persistence logic lives in `codelet_core::persistence`. This module
//! re-exports the public surface and hosts NAPI bindings (RPC-035).
pub use codelet_core::persistence::*;

#[cfg(not(feature = "noop"))]
mod napi_bindings;
#[cfg(not(feature = "noop"))]
pub use napi_bindings::*;
```

`codelet/napi/src/persistence/storage.rs` is deleted (RPC-032 already moved `MessageStore`).
`codelet/napi/src/persistence/types.rs` is deleted (everything re-exported via `codelet_core`).

## RPC-026 inverted-dependency note

`codelet/napi/src/persistence/mod.rs::delete_session` currently delegates the final disk-delete to `codelet_core::persistence::delete_session` (RPC-026). After this card, the whole `delete_session` lives in core; the inversion is no longer needed and can be inlined.

## Audit — call sites

- `codelet/napi/src/session_manager.rs:12-15,4217,4228,4273` — already imports `load_session, append_message_with_metadata, update_session_tokens, get_session_messages_full, update_message_metadata`. Re-exports keep these working.
- `codelet/napi/src/agent_manager_handler.rs:134,645,656,701,728,739` — `save_session, load_session, get_session_messages_full`.
- `codelet/napi/src/session_search_handler.rs:28-30,91,171` — `get_session_messages_full, is_blob_reference, load_session, StoredMessage, list_sessions_for_project`.
- `codelet/napi/src/test_support.rs:5-8,21,271` — `append_message_with_metadata, create_session, set_compaction_state, SessionManifest, set_data_directory, append_message`.
- `codelet/napi/tests/session_persistence_test.rs` (23 tests).
- `codelet/napi/tests/subordinate_session_persistence_test.rs:16-19` (4 tests).

## Acceptance criteria

1. `codelet/core/src/persistence/manifest.rs` contains `SessionStore` + `SessionManifest` + `SessionLineage` + every free function listed above.
2. `codelet/napi/src/persistence/storage.rs` and `types.rs` are deleted (or are 1-line re-export shims).
3. `codelet/napi/src/persistence/mod.rs` reduces to a thin facade (≤ 30 lines + bindings).
4. `cargo build -p codelet-core` and `-p codelet-napi` pass.
5. All 80+ persistence tests pass (`persistence::tests` 48, `lazy_init_tests` 9, `session_persistence_test` 23, `subordinate_session_persistence_test` 4).
6. Byte-identical round-trip of `session.json` and `messages.jsonl`.
7. RPC-026's inverted-call from NAPI back into core is no longer needed; delete the indirection.

## Risks

- The `lazy_static! Mutex<Option<...>>` singletons (`MESSAGE_STORE`, `SESSION_STORE`, `BLOB_STORE`, `HISTORY_STORE`, `DATA_DIRECTORY`) MUST move with the facade — they encode shared global state that both frontends rely on. Tests serialise on `TEST_MUTEX` in `tests.rs:setup_test_env()`.
- `pub(crate) fn list_sessions` becomes `pub fn` after the move (no other crate-internal reason for it to stay private).

## Out of scope

- BlobStore lift → RPC-034.
- NAPI bindings thin shims → RPC-035.
