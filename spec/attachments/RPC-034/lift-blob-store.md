# RPC-034 — Lift `BlobStore` into `codelet-core::persistence::blob`

**Parent:** RPC-030 · **Phase:** 1.4 · **Estimate:** 3 pts · **Depends on:** RPC-033

## Goal

Move blob storage (SHA-256 content-addressed file store + envelope blob processing) out of `codelet-napi` into `codelet-core::persistence::blob`. This is the last piece of pure-Rust persistence logic still living in NAPI.

## Source locations

### `codelet/napi/src/persistence/blob.rs` — 5,810 bytes, 186 lines

```rust
pub struct BlobStore { blobs_dir: PathBuf }
impl BlobStore {
    pub fn new() -> Result<Self, String>;
    pub fn store(&self, content: &[u8]) -> Result<String, String>; // SHA-256
    pub fn get(&self, hash: &str) -> Result<Vec<u8>, String>;
    pub fn exists(&self, hash: &str) -> bool;
    pub fn delete(&self, hash: &str) -> Result<(), String>;
    pub fn total_size(&self) -> Result<u64, String>;
}
pub fn should_use_blob_storage(content: &[u8]) -> bool; // uses BLOB_THRESHOLD const
```

Uses `super::{ensure_directories, get_data_dir}` — those moved in RPC-033, so just update the import path.

### `codelet/napi/src/persistence/blob_processing.rs` — 9,590 bytes, 227 lines

Pure-Rust (no NAPI). Public surface:

```rust
pub const BLOB_REF_PREFIX: &str = "blob:sha256:";
pub fn is_blob_reference(s: &str) -> bool;
pub fn extract_blob_hash(s: &str) -> Option<&str>;
pub fn make_blob_reference(hash: &str) -> String;
pub fn process_envelope_for_blob_storage(envelope: &MessageEnvelope)
    -> Result<(MessageEnvelope, Vec<(String, String)>), String>;
pub fn rehydrate_envelope_blobs(envelope_json: &str) -> Result<String, String>;
```

Imports `MessageEnvelope, MessagePayload, UserContent, AssistantContent, DocumentSource, ImageSource` (all lifted in RPC-031), plus `get_blob, should_use_blob_storage, store_blob` (free functions lifted in RPC-033).

## Target location

`codelet/core/src/persistence/blob.rs` + `codelet/core/src/persistence/blob_processing.rs`.

Update `codelet/core/src/persistence/mod.rs`:
```rust
pub mod blob;
pub mod blob_processing;
pub use blob::*;
pub use blob_processing::*;
```

## NAPI re-export

`codelet/napi/src/persistence/blob.rs` and `blob_processing.rs` are deleted. The umbrella `mod.rs` already re-exports everything from `codelet_core::persistence::*` (after RPC-033).

## Resolve the RPC-031 forward-reference

In RPC-031, the `#[cfg(test)]` block at `message_envelope.rs:270` was left with a `crate::persistence::should_use_blob_storage` reference. After this card, that becomes `codelet_core::persistence::should_use_blob_storage` (or just `super::should_use_blob_storage` if the test stays in the moved file).

## Audit — call sites

| Caller | Use |
|---|---|
| `napi/src/session_manager.rs` (indirect via session_search_handler) | `get_blob, is_blob_reference` |
| `napi/src/session_search_handler.rs:482,492` | `get_blob` |
| `napi/src/persistence/blob_processing.rs` | self-internal |
| `napi/src/persistence/mod.rs` | wraps `store_blob`, `get_blob`, `blob_exists` |
| `napi/src/persistence/tests.rs` | 48 tests cover blob round-trips + dedup |
| `napi/tests/session_persistence_test.rs` | indirect |

## Acceptance criteria

1. `BlobStore` + `should_use_blob_storage` live in `codelet/core/src/persistence/blob.rs`.
2. Envelope blob helpers live in `codelet/core/src/persistence/blob_processing.rs`.
3. `codelet/napi/src/persistence/blob.rs` and `blob_processing.rs` are deleted.
4. `cargo build -p codelet-core` + `-p codelet-napi` pass.
5. Blob round-trip tests in `persistence/tests.rs` pass — particularly the dedup test (store same 5MB blob twice → assert hash matches and disk has one file).
6. Hash-on-disk format unchanged (existing blob files still resolvable).
7. `BLOB_THRESHOLD` constant value preserved exactly.

## Risks

- `BLOB_THRESHOLD` (typically 10KB or so) is an implementation detail but changing it alters which messages get blob-extracted — keep the constant value byte-identical.
- `BLOB_REF_PREFIX = "blob:sha256:"` is on-wire — DO NOT rename.

## Out of scope

- NAPI binding shim cleanup → RPC-035.
