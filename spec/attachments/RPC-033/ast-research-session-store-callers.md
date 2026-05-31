# RPC-033 — AST research: who imports SessionStore / SessionManifest / session-level free functions from codelet-napi

Generated with `AstGrep` + `Grep` patterns over `codelet/napi/src/**.rs` and `codelet/napi/tests/**.rs` to confirm every external touchpoint of the symbols being lifted in this card.

## Symbols being lifted

### From `codelet/napi/src/persistence/storage.rs`
- `SessionStore` (struct + impl with `new`, `create`, `create_with_provider`, `save`, `get`, `get_mut`, `load`, `get_last_session`, `resume_last`, `list_for_project`, `list_all`, `delete`, `rename`, `fork`)

### From `codelet/napi/src/persistence/types.rs`
- `TokenUsage`, `MergeRecord`, `PastedContent`, `ForkPoint`, `CompactionState`
- `SessionManifest` (with `new`, `with_provider`, `add_message`, `record_merge`, `message_count`, `update_token_usage`)
- (The lifted-from-RPC-031 `HistoryEntry` re-export already lives here unchanged.)
- (RPC-032 already lifted `MessageRef`, `MessageSource`, `StoredMessage` to `codelet_core::persistence::messages`; the re-exports in `types.rs` stay until this whole file is deleted.)

### From `codelet/napi/src/persistence/mod.rs`
- `MESSAGE_STORE`, `SESSION_STORE` lazy_static singletons (lines 47-48; `BLOB_STORE` stays in NAPI until RPC-034)
- `init_message_store`, `init_session_store` (lines 114-129)
- `create_session`, `create_session_with_provider`, `save_session`, `load_session`, `resume_last_session`,
  `fork_session`, `merge_messages`, `cherry_pick`, `list_sessions` (currently `pub(crate)`), `switch_session`,
  `delete_session`, `rename_session`, `append_message`, `append_message_with_metadata`,
  `update_message_metadata`, `cleanup_orphaned_messages`, `get_message`, `get_session_messages`,
  `get_session_messages_full`, `update_session_tokens`, `set_session_tokens`, `set_compaction_state`,
  `clear_compaction_state`, `get_session_lineage`, `list_sessions_for_project`, `list_all_sessions`,
  `SessionLineage` (struct)

## Internal NAPI callers (must continue to compile via flat re-export `pub use codelet_core::persistence::*;`)

### `codelet/napi/src/persistence/mod.rs`
- Becomes a ~50-line thin facade. Every free function above MOVES — the file then re-exports `codelet_core::persistence::*` to keep callers' import paths intact.
- The defensive `codelet_core::persistence::delete_session(id)` double-delete at line 380 is REMOVED (the lifted facade is the canonical path).
- `set_data_directory` (lines 59-86) STAYS in NAPI because it resets credentials + graph + blob, but it delegates persistence-store resets to a new `codelet_core::persistence::reset_stores_for_tests` helper.
- `ensure_directories` (lines 96-111) is reduced to creating `blobs/` only — `messages/` is created lazily by RPC-032's `MessageStore::new()`, and `sessions/` is created lazily by the lifted `SessionStore::new()`.

### `codelet/napi/src/persistence/storage.rs`
- DELETED. The current re-export shim `pub use codelet_core::persistence::messages::{compute_hash, MessageStore};` (from RPC-032) moves into `codelet/napi/src/persistence/mod.rs` (or is absorbed by the flat `pub use codelet_core::persistence::*;`).

### `codelet/napi/src/persistence/types.rs`
- DELETED. Every type it currently re-exports (or owns) lives in `codelet_core::persistence::manifest` or `codelet_core::persistence::messages`.

### `codelet/napi/src/persistence/napi_bindings.rs`
- 859 lines, every `#[napi]` function calls into the lifted free functions through the flat re-export. No code change required — `use super::*;` continues to resolve via the new shim.
- `impl From<SessionManifest> for NapiSessionManifest` (and the inverse) continues to work because `SessionManifest` keeps the same field layout post-lift.

### `codelet/napi/src/persistence/tests.rs`
- 2,259 lines, uses `super::*` (line 12) — picks up the lifted names transparently. `TEST_MUTEX` stays where it is. The tests are exercised against the lifted facade via the shim.

### `codelet/napi/src/persistence/lazy_init_tests.rs`
- 396 lines. Currently reaches into `MESSAGE_STORE` and `SESSION_STORE` lazy_static globals directly via `super::*` (lines 28-39). After the lift, these accessor helpers switch to:
  - `codelet_core::persistence::is_message_store_initialized_for_tests()`
  - `codelet_core::persistence::is_session_store_initialized_for_tests()`
- Following the precedent set by `is_history_store_initialized` at line 41-43 which already routes through `codelet_core::persistence::history::is_initialized_for_tests()`.
- `BLOB_STORE` initialisation accessor stays untouched (blob lift is RPC-034).

### `codelet/napi/src/session_manager.rs`
- Line 12-15: `use crate::persistence::{ load_session, append_message_with_metadata, update_session_tokens, MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent };` — `load_session`, `append_message_with_metadata`, `update_session_tokens` resolve through the flat re-export.
- Lines 4032, 4055, 4072, 4116, 4133, 4171, 4184, 4187, 4217, 4228, 4273: `load_session`, `append_message_with_metadata`, `update_session_tokens`, `get_session_messages_full`, `update_message_metadata` calls — all unchanged.

### `codelet/napi/src/session_search_handler.rs`
- Line 28-30: `use crate::persistence::{ self, get_session_messages_full, is_blob_reference, load_session, StoredMessage };` — all resolve through the flat re-export.
- Line 91, 162, 171: `persistence::list_sessions_for_project`, `persistence::list_all_sessions` — unchanged.
- Line 427, 461, 481: `crate::persistence::SessionManifest`, `crate::persistence::extract_blob_hash` — `SessionManifest` resolves through the new shim; `extract_blob_hash` is in `blob_processing` (RPC-034 territory) and stays in NAPI for now.
- Line 482, 492: `persistence::get_blob(...)` — stays in NAPI (RPC-034).

### `codelet/napi/src/agent_manager_handler.rs`
- Line 126, 1195: `crate::persistence::SessionManifest::with_provider(...)` — resolves through the new shim.
- Line 134: `crate::persistence::save_session(&manifest)` — unchanged.
- Line 632, 645, 656, 701, 728, 739: `use crate::persistence;` + `persistence::load_session`, `persistence::get_session_messages_full` — unchanged.

### `codelet/napi/src/test_support.rs`
- Line 5-8: `use crate::persistence::{ append_message_with_metadata, create_session, set_compaction_state, SessionManifest };` — all resolve through the new shim.
- Line 21: `crate::persistence::set_data_directory(temp_dir.path().to_path_buf())` — `set_data_directory` STAYS in NAPI (wraps the core reset + credentials + graph + blob resets). Unchanged.
- Line 271: `use crate::persistence::append_message;` — unchanged.

### `codelet/napi/src/lib.rs`
- Line 90: `pub use persistence::*;` — flat NAPI-level re-export continues to expose the symbols at `codelet_napi::persistence::*`.

### `codelet/napi/src/credentials/store.rs`
- Line 127: doc comment referring to `persistence::set_data_directory`. No code change.

## NAPI test files (`codelet/napi/tests/`)

| File | Imports lifted symbols? |
|---|---|
| `session_persistence_test.rs` | 23 tests using `append_message_with_metadata`, `load_session`, `create_session`, `fork_session`, `merge_messages`, `cherry_pick`, `set_compaction_state`, `update_session_tokens` — all via the shim. |
| `subordinate_session_persistence_test.rs` | 4 tests using `SessionManifest`, `save_session`, `load_session` — all via the shim. |
| `session_restore_messages_test.rs`, `session_token_restore_test.rs`, `session_pause_state_test.rs`, etc. | All consume `codelet_napi::persistence::*` — unchanged via the shim. |

## codelet-rpc-embedded gate

`codelet/rpc-embedded/tests/rpc_006_source_shape.rs` already enforces the absence of `rpc → napi` arrows. After this lift, downstream RPC code (and the future `codelet-sessions` crate) can:

```rust
use codelet_core::persistence::{
    SessionStore, SessionManifest, load_session, append_message_with_metadata,
    fork_session, merge_messages, cherry_pick, update_session_tokens,
    set_compaction_state, clear_compaction_state,
};
```

…without re-introducing the forbidden arrow.

## RPC-026 cross-transport delete

`codelet/fspec-tui/tests/rpc026_cross_transport_parity.rs` (line 212) calls `codelet_core::persistence::sessions::delete_session(s2)` directly. The lifted `codelet_core::persistence::delete_session` becomes the canonical path; the existing `sessions.rs` file's `delete_session` is either:
- Folded into `manifest.rs` as the lifted `SessionStore::delete` (and re-exported via `pub fn delete_session(id: Uuid) -> Result<(), String>` at the module root), or
- Kept as a thin alias `pub use crate::persistence::manifest::delete_session;` to avoid breaking the parity test.

Either way, the test continues to pass.

## TS frontend (`codelet/napi/index.d.ts`)

`napi_bindings.rs` continues to expose `NapiSessionManifest`, `persistence_create_session`, `persistence_load_session`, `persistence_append_message_with_metadata`, etc. with the same TS signatures. The Rust→NAPI conversions (`impl From<SessionManifest> for NapiSessionManifest`) keep working because `SessionManifest`'s field order and serde derives are preserved by the move.

## Conclusion

Every external symbol use is either:
1. Resolved transparently via the flat re-export `pub use codelet_core::persistence::*;` in `napi/src/persistence/mod.rs`, or
2. A single internal helper (`lazy_init_tests.rs::is_*_initialized`) that switches from `MESSAGE_STORE.lock()...is_some()` to the new `codelet_core::persistence::is_*_initialized_for_tests()` accessors, matching the precedent set by `history::is_initialized_for_tests()`.

No call site in `session_manager.rs`, `session_search_handler.rs`, `agent_manager_handler.rs`, or `test_support.rs` requires a code change. The 80+ persistence tests in `codelet-napi` continue to run against the same symbols, now resolved through the lifted location.
