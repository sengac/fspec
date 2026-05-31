# RPC-032 — AST research: who imports MessageStore / StoredMessage / MessageRef / message_index from codelet-napi

Generated with `AstGrep` patterns over `codelet/napi/src/**.rs` to confirm every external touchpoint of the symbols being lifted.

## Symbols being lifted

- `MessageStore` (in `napi/src/persistence/storage.rs`)
- `compute_hash` (in `napi/src/persistence/storage.rs`)
- `StoredMessage`, `MessageSource`, `MessageRef` (in `napi/src/persistence/types.rs`)
- `IndexEntry`, `load_index`, `save_index`, `scan_jsonl_range`, `read_message_at` (in `napi/src/persistence/message_index.rs`)

## Internal NAPI callers (must continue to compile via re-export shims)

### `codelet/napi/src/persistence/storage.rs`
- `use super::message_index::{self, IndexEntry};` — replaced when the file no longer holds `MessageStore`. SessionStore (which stays in this file) does not use `message_index`. Line goes away.
- `use super::types::*;` — replaced with explicit imports of just what SessionStore needs: `SessionManifest`, `ForkPoint`, `MessageRef`, `MessageSource`. After lift the latter two come transparently through the types.rs re-export.

### `codelet/napi/src/persistence/mod.rs`
- `static ref MESSAGE_STORE: Mutex<Option<MessageStore>>` (line 48) — uses the re-exported `MessageStore`.
- `*store = Some(MessageStore::new()?);` (line 118) — same.
- `let referenced = msg_store_ref.get_referenced_ids(&sessions);` (line 520) — the lifted `MessageStore` no longer exposes `get_referenced_ids` (it required `&[SessionManifest]` which is NAPI-only). The caller inlines the trivial HashSet build.
- `pub use storage::*;` and `pub use types::*;` (lines 34-35) — both keep working because storage.rs re-exports `MessageStore` + `compute_hash`, and types.rs re-exports `StoredMessage` + `MessageSource` + `MessageRef`.
- `target.messages.push(MessageRef { ... });` + `source: MessageSource::Imported { .. }` (lines 246, 248, 308, 310) — work via the re-export.
- `session.add_message(msg_id, MessageSource::Native);` (lines 411, 442) — same.
- `pub fn get_message(...) -> Result<Option<StoredMessage>, String>` (line 552) — same.
- `pub fn get_session_messages(session: &SessionManifest) -> Result<Vec<StoredMessage>, String>` (line 569) — same.
- `messages.push(StoredMessage { ... });` (line 585) — same.

### `codelet/napi/src/persistence/napi_bindings.rs`
- `impl From<StoredMessage> for NapiStoredMessage` (line 522) — works via the re-export.

### `codelet/napi/src/persistence/tests.rs`
- `use super::*;` (line 12) — picks up the re-exported names.
- `MessageSource::Imported { from_session, .. }` (line 134) — works via the re-export.

### `codelet/napi/src/persistence/lazy_init_tests.rs`
- `use super::*;` (line 14) and direct references to `MESSAGE_STORE` (lines 28-32) — both unaffected because the singleton stays in `napi/src/persistence/mod.rs`.

### `codelet/napi/src/session_search_handler.rs`
- `use crate::persistence::{ self, get_session_messages_full, is_blob_reference, load_session, StoredMessage };` (line 28) — `StoredMessage` resolves through the types.rs re-export.

### `codelet/napi/src/session_manager.rs`
- `use crate::persistence::{ load_session, append_message_with_metadata, update_session_tokens, MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent };` (line 12) — does NOT touch any of the symbols being lifted in this card. The `MessageEnvelope` family was already lifted by RPC-031. No change required here.

### `codelet/napi/src/test_support.rs`
- `use crate::persistence::{ append_message_with_metadata, create_session, set_compaction_state, SessionManifest };` (line 5) — does NOT touch any of the lifted-in-RPC-032 symbols.

## External NAPI consumers (TS via `index.d.ts`)

`napi_bindings.rs` exposes `NapiStoredMessage` to TS. The Rust→NAPI conversion (`impl From<StoredMessage> for NapiStoredMessage`) is unchanged because `StoredMessage` keeps the same field order and serde derives — only its source location moves.

## codelet-rpc-embedded gate

`codelet/rpc-embedded/tests/rpc_006_source_shape.rs` already enforces the absence of `rpc → napi` arrows. After the lift, downstream RPC code (and the future `codelet-sessions` crate) can `use codelet_core::persistence::messages::{MessageStore, StoredMessage}` without re-introducing the forbidden arrow.

## Conclusion

Every external symbol use is either:
1. Resolved transparently via the re-export shims in `storage.rs` (for `MessageStore`, `compute_hash`) and `types.rs` (for `StoredMessage`, `MessageSource`, `MessageRef`), or
2. A single internal call (`MessageStore::get_referenced_ids` in `mod.rs::cleanup_orphaned_messages`) that we inline at the call site since `get_referenced_ids` was a one-line iterator over SessionManifests and we want the lifted `MessageStore` to have zero `SessionManifest` dependency.

No other files require source changes.
