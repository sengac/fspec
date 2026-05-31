# RPC-034 AST Research — BlobStore + blob_processing caller audit

## Goal
Catalogue every call site of `BlobStore`, the `BLOB_STORE` singleton, the free-function blob facade (`store_blob`, `get_blob`, `blob_exists`, `init_blob_store`), the envelope blob helpers (`BLOB_REF_PREFIX`, `is_blob_reference`, `extract_blob_hash`, `make_blob_reference`, `process_envelope_for_blob_storage`, `rehydrate_envelope_blobs`), the `should_use_blob_storage` predicate, and the local `ensure_directories` helper across the codelet workspace so the lift in RPC-034 (Phase 1.4) can move them out of `codelet-napi` without breaking any caller.

## Tooling
- AstGrep over `codelet/napi/src/persistence/blob.rs` for `pub fn $NAME($$$ARGS) -> Result<$T, String> { $$$BODY }`
- AstGrep over `codelet/napi/src/persistence/blob_processing.rs` for `pub fn $NAME($$$ARGS) -> $T { $$$BODY }`
- Ripgrep over the workspace for usages of every symbol on the surface

## Public surface (to be lifted)

### `codelet/napi/src/persistence/blob.rs` (186 lines, 5,810 bytes)

| Symbol | Kind | Line | Notes |
|---|---|---|---|
| `BlobStore` | struct | 13 | sole field is `blobs_dir: PathBuf` |
| `BlobStore::new` | fn | 19 | currently calls `super::{ensure_directories, get_data_dir}` |
| `BlobStore::store` | fn | 28 | atomic write via `.tmp` + rename |
| `BlobStore::get` | fn | 54 | hex-validates the 64-char hash before slicing |
| `BlobStore::exists` | fn | 86 | also hex-validates |
| `BlobStore::delete` | fn | 95 | idempotent on missing files |
| `BlobStore::get_blob_path` | fn (private) | 108 | first-2-hex subdir + full hash filename |
| `BlobStore::total_size` | fn | 120 | walks the two-level dir structure |
| `compute_sha256` | fn (private) | 147 | helper for store/get |
| `should_use_blob_storage` | fn | 157 | uses `BLOB_THRESHOLD = 10 * 1024` |
| `tests::test_compute_sha256` | test | 168 | known SHA-256 of `"hello world"` |
| `tests::test_should_use_blob_storage` | test | 179 | 100 byte vs 20000 byte threshold |

### `codelet/napi/src/persistence/blob_processing.rs` (227 lines, 9,590 bytes)

| Symbol | Kind | Line | Notes |
|---|---|---|---|
| `BLOB_REF_PREFIX` | const | 13 | `"blob:sha256:"` — wire format |
| `is_blob_reference` | fn | 16 | checks prefix + 64-char hash |
| `extract_blob_hash` | fn | 21 | returns `Option<&str>` slice |
| `make_blob_reference` | fn | 30 | format `"blob:sha256:{hash}"` |
| `maybe_store_blob` | fn (private) | 36 | calls `should_use_blob_storage` + `store_blob` |
| `process_envelope_for_blob_storage` | fn | 50 | matches User/Assistant + content variants |
| `rehydrate_envelope_blobs` | fn | 131 | reverses process_envelope_for_blob_storage |
| `tests::test_blob_reference_format` | test | 207 | valid/invalid prefix combinations |

### `codelet/napi/src/persistence/mod.rs` (current, to shrink)

| Symbol | Kind | Line | Notes |
|---|---|---|---|
| `BLOB_STORE` | `lazy_static! Mutex<Option<BlobStore>>` | 49–51 | global singleton |
| `set_data_directory` | fn | 66 | currently resets BLOB_STORE inline (lines 73–76) |
| `ensure_directories` | fn | 105 | creates `messages/`, `sessions/`, `blobs/` |
| `init_blob_store` | fn (private) | 115 | lazy init pattern |
| `store_blob` | fn | 124 | thin wrapper around `BLOB_STORE.lock().unwrap().store(...)` |
| `get_blob` | fn | 134 | same pattern |
| `blob_exists` | fn | 144 | same pattern |

## Caller audit (intra-NAPI + workspace)

### `codelet/napi/src/persistence/napi_bindings.rs`
- L241–242 `persistence_store_blob` — calls free fn `store_blob`
- L247–251 `persistence_get_blob` — calls free fn `get_blob`
- L255–256 `persistence_blob_exists` — calls free fn `blob_exists`
- L580–583 `use super::blob_processing::{process_envelope_for_blob_storage as process_envelope_impl, rehydrate_envelope_blobs as rehydrate_envelope_impl};` — used by `persistence_store_message_envelope` (L590+) and `persistence_get_message_envelope` (L652+)

After the lift: the `super::blob_processing::` path no longer resolves (the module is deleted). Rewrite to `use crate::persistence::{process_envelope_for_blob_storage as process_envelope_impl, rehydrate_envelope_blobs as rehydrate_envelope_impl};` (resolves through the flat re-export of `codelet_core::persistence::*`).

### `codelet/napi/src/persistence/tests.rs`
17 references to `super::blob_processing::{extract_blob_hash, is_blob_reference, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs}` across the 12 blob-coverage tests:

| Line | Use |
|---|---|
| 886 | `use super::blob_processing::{extract_blob_hash, is_blob_reference, make_blob_reference};` (test_blob_reference_format) |
| 935 | `super::blob_processing::process_envelope_for_blob_storage(&envelope)` (test_tool_result_blob_storage_and_rehydration) |
| 975 | `super::blob_processing::rehydrate_envelope_blobs(&processed_json)` (same) |
| 1023 | `process_envelope_for_blob_storage` (test_image_blob_storage_and_rehydration) |
| 1051 | `rehydrate_envelope_blobs` (same) |
| 1104 | `process_envelope_for_blob_storage` (test_document_blob_storage_and_rehydration) |
| 1148 | `rehydrate_envelope_blobs` (same) |
| 1201 | `process_envelope_for_blob_storage` (test_thinking_content_blob_storage) |
| 1234 | `rehydrate_envelope_blobs` (same) |
| 1281 | `process_envelope_for_blob_storage` (test_tool_use_input_blob_storage_with_marker) |
| 1345 | `process_envelope_for_blob_storage` (test_blob_storage_threshold) |
| 1428, 1430 | `process_envelope_for_blob_storage` (test_blob_storage_dedup) |
| 1485 | `process_envelope_for_blob_storage` (test_multi_content_envelope_dedup) |
| 1520 | `rehydrate_envelope_blobs` (same) |
| 1576 | `process_envelope_for_blob_storage` (test_thinking_with_signature_blob_storage) |
| 1628 | `process_envelope_for_blob_storage` (test_text_content_not_blobified) |
| 1682 | `process_envelope_for_blob_storage` (test_url_image_not_blobified) |

After the lift each `super::blob_processing::` is rewritten to `super::` (the flat re-export covers it) — pure mechanical substitution; no semantics change.

Also at L350, 361, 416, 419, 966 the tests call `store_blob` / `get_blob` directly via `super::` — these continue to resolve via the re-export.

### `codelet/napi/src/persistence/lazy_init_tests.rs`
L40–44 currently reads `BLOB_STORE.lock().map(|s| s.is_some()).unwrap_or(false)`. After the lift the global singleton lives in codelet-core; this helper becomes:

```rust
fn is_blob_store_initialized() -> bool {
    codelet_core::persistence::is_blob_store_initialized_for_tests()
}
```

L83–115 (`test_lazy_store_message_inits_message_and_blob_and_session_store`) asserts BLOB_STORE remains uninitialised after `append_message` — same assertion holds against the lifted accessor.

### `codelet/napi/src/session_search_handler.rs`
- L481 `if let Some(hash) = crate::persistence::extract_blob_hash(&msg.content)` → resolves via re-export after the lift.
- L482, L492 `persistence::get_blob(hash)` and `persistence::get_blob(blob_ref)` → resolve via re-export.

Zero source changes required.

### `codelet/napi/src/session_manager.rs`
- L5120 — comment only ("The handler accesses the persistence layer directly (MessageStore, SessionStore, BlobStore)") — update copy to reflect that BlobStore now lives in codelet-core if a doc-comment refresh is desired, otherwise harmless.

### `codelet/core/src/persistence/message_envelope.rs`
- L270 — comment near `#[cfg(test)]` block left over from RPC-031:
> `crate::persistence::should_use_blob_storage` stays in the NAPI shim
> (`napi/src/persistence/message_envelope.rs`) because `should_use_blob_storage` still lives in `napi::persistence::blob` until RPC-034.

Resolution: rewrite or remove the comment. The `test_blob_threshold` test can move to `codelet/core/src/persistence/blob.rs` (alongside the lifted `should_use_blob_storage`) or stay as-is — pick whichever needs fewer source changes; the existing in-NAPI `test_should_use_blob_storage` at `blob.rs:179` already covers the same invariant, so the easiest path is to delete the stale comment without moving the test.

### `codelet/napi/tests/message_envelope_lift_shim_test.rs`
- L178, L184–188 — asserts `use codelet_napi::persistence::should_use_blob_storage;` still resolves through the NAPI flat re-export. After the lift this still resolves (the re-export still names `should_use_blob_storage`, now coming from `codelet_core::persistence::blob`). Zero source changes required.

### Workspace consumers outside codelet-napi
A workspace-wide grep for `use codelet_(core|napi)::persistence::{...blob...}` returns:

- `codelet/napi/tests/message_envelope_lift_shim_test.rs:184` (already covered above)

No other crate currently consumes the blob surface. The lift adds a new `codelet/core/tests/blob_store_lifted_test.rs` integration test as the first downstream consumer (proving the surface is reachable from outside codelet-napi).

### Dependency-rule guards
- `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — currently enforces the forbidden `rpc → napi` arrow. After the lift, codelet-rpc-embedded can `use codelet_core::persistence::{...blob...}` and the test continues to pass.

## Plan derived from this audit

1. Move `codelet/napi/src/persistence/blob.rs` → `codelet/core/src/persistence/blob.rs`. Replace `use super::{ensure_directories, get_data_dir};` with a direct `codelet_common::get_data_dir()?` + `fs::create_dir_all({data_dir}/blobs)`. Pull the `BLOB_STORE` lazy_static + `init_blob_store` + `store_blob`/`get_blob`/`blob_exists` free-function facade into the same file. Add `is_blob_store_initialized_for_tests` + `reset_blob_store_for_tests` test-only accessors.
2. Move `codelet/napi/src/persistence/blob_processing.rs` → `codelet/core/src/persistence/blob_processing.rs`. The imports of `MessageEnvelope`/`MessagePayload`/`UserContent`/`AssistantContent`/`DocumentSource`/`ImageSource` already live in `codelet_core::persistence` (RPC-031). The `get_blob`/`should_use_blob_storage`/`store_blob` calls now resolve from the sibling module.
3. Update `codelet/core/src/persistence/mod.rs`: `pub mod blob; pub mod blob_processing; pub use blob::*; pub use blob_processing::*;`.
4. Widen `codelet/core/src/persistence/manifest.rs::reset_stores_for_tests` to also reset the BLOB_STORE singleton (or add a sibling `reset_blob_store_for_tests` in `blob.rs` and call it from `reset_stores_for_tests`).
5. Delete `codelet/napi/src/persistence/blob.rs` and `codelet/napi/src/persistence/blob_processing.rs`. Shrink `codelet/napi/src/persistence/mod.rs`: drop `mod blob; mod blob_processing;`, drop the `BLOB_STORE` lazy_static, drop `init_blob_store`/`store_blob`/`get_blob`/`blob_exists`, drop `ensure_directories` (verify no remaining callers — `BlobStore::new` was the only one, and it now lives in core), drop the BLOB_STORE-reset block inside `set_data_directory`. Keep only `set_data_directory` (delegating to `reset_stores_for_tests`), `get_data_dir`, the History wrappers, the `pub use codelet_core::persistence::*;` flat re-export, and the conditional `mod napi_bindings`/`mod tests`/`mod lazy_init_tests` declarations.
6. Rewrite the 17 `super::blob_processing::…` references in `codelet/napi/src/persistence/tests.rs` to `super::…`.
7. Rewrite the `use super::blob_processing::{…}` block in `codelet/napi/src/persistence/napi_bindings.rs` (L580–583) to use the flat re-export.
8. Rewrite `codelet/napi/src/persistence/lazy_init_tests.rs:40–44`'s direct `BLOB_STORE.lock()...` to use `codelet_core::persistence::is_blob_store_initialized_for_tests()`.
9. Resolve the stale `RPC-034` forward-reference comment at `codelet/core/src/persistence/message_envelope.rs:269–273` — either delete the comment or move the `test_blob_threshold` test to live next to the lifted `should_use_blob_storage`.
10. Add `codelet/core/tests/blob_store_lifted_test.rs` (proves the surface is reachable from outside codelet-napi).
11. Add `codelet/napi/tests/blob_store_lift_shim_test.rs` (asserts compile-time type identity through the re-export shim).
