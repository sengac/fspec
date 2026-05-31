# RPC-032 — Lift `MessageStore` + `message_index` into `codelet-core::persistence::messages`

**Parent:** RPC-030
**Phase:** 1.2
**Estimate:** 5 points
**Depends on:** RPC-031

---

## Goal

Move the on-disk message store (`messages.jsonl` + `messages.idx` binary index) out of `codelet-napi` into `codelet-core::persistence`. After this card both frontends can append/load messages without touching `napi`.

---

## Source locations (move from)

### `codelet/napi/src/persistence/storage.rs` — 21,362 bytes, 621 lines

`MessageStore` lives at **lines 31–349**. Public surface:

```rust
pub struct MessageStore { /* messages dir, in-memory index, cache */ }

impl MessageStore {
    pub fn new() -> Result<Self, String>;
    pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String>;
    pub fn store_with_metadata(&mut self, role, content, metadata) -> Result<Uuid, String>;
    pub fn get(&self, id: Uuid) -> Option<StoredMessage>;
    pub fn update_metadata(&mut self, id, metadata) -> Result<(), String>;
    pub fn get_referenced_ids(&self) -> HashSet<Uuid>;
    pub fn cleanup_orphans(&mut self, referenced: HashSet<Uuid>) -> Result<usize, String>;
    pub fn index_len(&self) -> usize;
    pub fn cache_len(&self) -> usize;
}

pub fn compute_hash(content: &[u8]) -> String;
```

`SessionStore` also lives in this file (lines 352–620) — that one moves in RPC-033, NOT this card.

### `codelet/napi/src/persistence/message_index.rs` — 7,302 bytes, 213 lines

Binary index for `messages.jsonl` (BUG-122 Layer 2). NOT re-exported by `mod.rs` — internal to `storage.rs`. Public surface:

```rust
pub struct IndexEntry { byte_offset: u64, byte_length: u32 }
pub fn load_index(index_path: &Path) -> Option<(HashMap<Uuid, IndexEntry>, u64)>;
pub fn save_index(index_path, entries, data_file_size) -> io::Result<()>;
pub fn scan_jsonl_range(file, start_offset, end_offset, on_message) -> io::Result<()>;
pub fn read_message_at(file, offset, length) -> io::Result<StoredMessage>;
```

### `codelet/napi/src/persistence/types.rs` — supporting types

- `StoredMessage` — needed by `MessageStore::get`
- `MessageSource` enum
- `MessageRef`

These should be lifted at the same time (they are zero-cost passthrough; the persistence-public types must all live together). Move into `codelet/core/src/persistence/types.rs`.

---

## Target location (move to)

`codelet/core/src/persistence/messages.rs` containing `MessageStore` + `message_index` as a sibling module:

```rust
// codelet/core/src/persistence/messages.rs
mod index; // formerly message_index.rs
pub use index::*; // or selectively

pub struct MessageStore { /* … */ }
impl MessageStore { /* … */ }

pub fn compute_hash(content: &[u8]) -> String;
```

Add `pub mod messages;` and `pub use messages::*;` to `codelet/core/src/persistence/mod.rs`.

---

## NAPI re-export shim

After the move, `codelet/napi/src/persistence/storage.rs` keeps only `SessionStore` (RPC-033 lifts that separately). For now, surgically delete the `MessageStore` block and replace with:

```rust
pub use codelet_core::persistence::{MessageStore, compute_hash};
```

`codelet/napi/src/persistence/message_index.rs` becomes:

```rust
pub use codelet_core::persistence::messages::{IndexEntry, load_index, save_index, scan_jsonl_range, read_message_at};
```

(Or delete it entirely and let `mod message_index;` drop from `persistence/mod.rs`. Either works — choose deletion since `mod.rs` does NOT re-export it.)

---

## Global singleton wiring

`codelet/napi/src/persistence/mod.rs` holds `lazy_static! Mutex<Option<MessageStore>>` (the `MESSAGE_STORE` singleton). For now keep the singleton in `mod.rs` and have it lock the moved type. After RPC-033 the whole `mod.rs` facade moves too.

---

## Audit — who calls these

| Call site | Notes |
|---|---|
| `persistence/mod.rs` free functions (`append_message`, `get_message`, `get_session_messages_full`, `cleanup_orphaned_messages`, `update_message_metadata`) | Internal — re-routes through `MESSAGE_STORE` lock |
| `napi/src/session_manager.rs:4217,4228,4273` | `crate::persistence::{load_session, get_session_messages_full, update_message_metadata}` |
| `napi/src/session_search_handler.rs:28-30` | `get_session_messages_full, StoredMessage` |
| `napi/src/agent_manager_handler.rs:134,645,656,701,728,739` | `save_session, load_session, get_session_messages_full` (via `use crate::persistence;`) |
| `napi/src/test_support.rs:5-8,21,271` | Used in test setup |
| `napi/tests/session_persistence_test.rs:34-37` | Public NAPI re-export consumer |

After this card, all of the above continue to work via the `pub use codelet_core::persistence::*` re-export from `napi/src/persistence/mod.rs`.

---

## Acceptance criteria

1. `MessageStore`, `compute_hash`, and the index helpers live in `codelet/core/src/persistence/messages.rs`.
2. `codelet/napi/src/persistence/message_index.rs` is deleted (or is a 1-line re-export shim).
3. `codelet/napi/src/persistence/storage.rs` no longer contains `MessageStore`; it still contains `SessionStore` (lifted in RPC-033).
4. `cargo build -p codelet-core` + `cargo build -p codelet-napi` both pass.
5. `cargo test -p codelet-napi persistence::tests` passes (48 tests).
6. `cargo test -p codelet-napi persistence::lazy_init_tests` passes (9 tests, BUG-122 coverage).
7. `cargo test -p codelet-napi --test session_persistence_test` passes (23 tests).
8. Re-run round-trip of an existing `messages.jsonl` + `messages.idx` — both files byte-identical for the same inputs (load existing data → assert in-memory == disk).
9. Boot the TS frontend against a session with thousands of messages; assert page-load latency unchanged (BUG-122 lazy-init still working).

---

## Risks & notes

- `MessageStore` and `SessionStore` are tangled in `storage.rs` (both in same file). Moving `MessageStore` first means `storage.rs` stays in NAPI with only `SessionStore` until RPC-033. Use clear file-section markers to avoid accidental deletions.
- The index file (`messages.idx`) is binary — DO NOT change format. Keep `IndexEntry` field order identical.
- BUG-122 lazy-init: `MessageStore::new()` does NOT scan the whole file at startup; it relies on the `.idx` file. Preserve this behaviour.
- Tests serialize on `TEST_MUTEX` in `persistence/tests.rs:setup_test_env()`. Move that fixture too if needed, or leave it in NAPI and have it import the core types.

---

## Out of scope

- `SessionStore` lift → RPC-033.
- `BlobStore` lift → RPC-034.
- NAPI binding thin-shim cleanup → RPC-035.
