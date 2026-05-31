# RPC-035 AST Research — codelet-napi persistence callers

Captured: 2026-05-20

## Goal of the research

Confirm the current shape of `codelet/napi/src/persistence/` and audit every caller of the helpers in `mod.rs` so RPC-035 (the Phase 1.5 cleanup card after the lift cards RPC-031..RPC-034) can:

1. Replace `use super::*;` in `napi_bindings.rs` with an explicit `use codelet_core::persistence::{...};` import block.
2. Inline the four remaining helpers from `persistence/mod.rs` (`set_data_directory`, `get_data_dir`, `ensure_directories`, `add_history_entry`, `get_history`, `search_history`) into the `#[napi]` functions in `napi_bindings.rs` or replace their callers with direct calls into `codelet_core::persistence`.
3. Relocate the in-crate test modules (`tests.rs` + `lazy_init_tests.rs`) from `codelet/napi/src/persistence/` to `codelet/core/src/persistence/`.

All searches were performed with the `AstGrep` and `Grep` tools (ripgrep) against the working tree.

## Findings

### A. Persistence directory contents (already minimal after RPC-034)

```text
-rw-r--r--  14680  2026-05-20 06:24  lazy_init_tests.rs
-rw-r--r--   4509  2026-05-20 06:54  mod.rs
-rw-r--r--  35422  2026-05-20 06:24  napi_bindings.rs
-rw-r--r--  89620  2026-05-20 06:24  tests.rs
```

After RPC-035 the target state is **two files only**: `mod.rs` (~15 lines) and `napi_bindings.rs` (~35 kB / 940 lines).

### B. `pub fn` items still owned by codelet-napi (must be inlined or deleted)

`AstGrep(rust, "pub fn $NAME($$$ARGS) -> $RET { $$$BODY }", path=codelet/napi/src/persistence)`

Returns 41 entries: 35 `persistence_*` `#[napi]` shims in `napi_bindings.rs` (these stay, they are the NAPI bridge) plus the 6 surviving helpers in `mod.rs`:

```text
codelet/napi/src/persistence/mod.rs:49:1:pub fn set_data_directory(dir: PathBuf) -> Result<(), String>
codelet/napi/src/persistence/mod.rs:68:1:pub fn get_data_dir() -> Result<PathBuf, String>
codelet/napi/src/persistence/mod.rs:83:1:pub fn ensure_directories() -> Result<(), String>
codelet/napi/src/persistence/mod.rs:97:1:pub fn add_history_entry(entry: HistoryEntry) -> Result<(), String>
codelet/napi/src/persistence/mod.rs:102:1:pub fn get_history(project: Option<&Path>, limit: Option<usize>) -> Result<Vec<HistoryEntry>, String>
codelet/napi/src/persistence/mod.rs:110:1:pub fn search_history(query: &str, project: Option<&Path>) -> Result<Vec<HistoryEntry>, String>
```

All six must be either inlined into the matching `#[napi]` entry point in `napi_bindings.rs` or deleted (callers switch to `codelet_core::persistence::*` directly).

### C. `use super::*` and `super::X` references in the persistence module

```text
codelet/napi/src/persistence/napi_bindings.rs:5:use super::*;
codelet/napi/src/persistence/napi_bindings.rs:583:use super::{process_envelope_for_blob_storage as process_envelope_impl, rehydrate_envelope_blobs as rehydrate_envelope_impl};
codelet/napi/src/persistence/lazy_init_tests.rs:14:use super::*;
codelet/napi/src/persistence/lazy_init_tests.rs:15:use super::tests::TEST_MUTEX;
codelet/napi/src/persistence/tests.rs:12:use super::*;
```

Plus 18 inline references to `super::MessagePayload` / `super::UserContent` / `super::AssistantContent` inside the helper free functions (`calculate_envelope_tokens` and `extract_content_summary` at lines 857-940 of `napi_bindings.rs`):

```text
codelet/napi/src/persistence/napi_bindings.rs:863:9:super::MessagePayload
codelet/napi/src/persistence/napi_bindings.rs:866:21:super::UserContent
... (16 more identical type-path references)
```

After RPC-035 these all become unqualified names (resolved via the explicit `use codelet_core::persistence::{...};` import at the top of `napi_bindings.rs`).

### D. External callers of `crate::persistence::{set_data_directory, ensure_directories, add_history_entry, get_history, search_history}`

```text
codelet/napi/src/test_support.rs:21      crate::persistence::set_data_directory(temp_dir.path().to_path_buf())
codelet/napi/src/persistence/napi_bindings.rs:19   set_data_directory(PathBuf::from(dir)).map_err(...)   # via use super::*
codelet/napi/src/persistence/napi_bindings.rs:25   get_data_dir()                                          # via use super::*
codelet/napi/src/persistence/napi_bindings.rs:268  add_history_entry(entry)                                # via use super::*
codelet/napi/src/persistence/lazy_init_tests.rs:22 set_data_directory(temp_dir.path().to_path_buf())       # via use super::*
codelet/napi/src/persistence/lazy_init_tests.rs:246 add_history_entry(HistoryEntry::new(...))             # ditto
codelet/napi/src/persistence/lazy_init_tests.rs:279 add_history_entry(HistoryEntry::new(...))             # ditto
codelet/napi/src/persistence/lazy_init_tests.rs:286 add_history_entry(HistoryEntry::new(...))             # ditto
codelet/napi/src/persistence/lazy_init_tests.rs:294 add_history_entry(HistoryEntry::new(...))             # ditto
codelet/napi/src/persistence/tests.rs:32  set_data_directory(temp_dir.path().to_path_buf())                # ditto
codelet/napi/src/persistence/tests.rs:191 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:197 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:292 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:376 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:387 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:659 add_history_entry(HistoryEntry::new(...))                       # ditto
codelet/napi/src/persistence/tests.rs:668 add_history_entry(HistoryEntry::new(...))                       # ditto
```

**Implications:**

- `test_support.rs:21` — the ONLY external caller of the napi-local `set_data_directory` outside the persistence module. Must be rewritten to inline the four-line reset sequence (`codelet_common::set_data_directory(...)` + `codelet_core::persistence::reset_stores_for_tests()` + `crate::credentials::reset_credential_store()` + `crate::graph::reset_graph_db()`).
- All `add_history_entry` / `set_data_directory` references inside `tests.rs` + `lazy_init_tests.rs` go away once those files move to codelet-core (where they call `codelet_core::persistence::history::add` and `codelet_common::set_data_directory` + `reset_stores_for_tests` directly).
- `crate::persistence::ensure_directories` has zero callers outside the directory — confirmed by `Grep('ensure_directories', codelet/napi/src)` returning only the def in `mod.rs:83`. **Safe to delete** the mod.rs shim outright.

### E. `#[napi]` count baseline (TS surface snapshot)

`AstGrep(rust, "pub fn $NAME($$$ARGS) -> $RET { $$$BODY }", path=codelet/napi/src/persistence/napi_bindings.rs)` returns 35 `pub fn persistence_*` entries plus 6 helper functions (`truncate_chars`, `calculate_envelope_tokens`, `extract_content_summary` — these are NOT `#[napi]`).

The `index.d.ts` byte-identity rule requires:

- All 35 `persistenceXxx` exports remain
- All 9 `Napi*` interface declarations remain (NapiSessionManifest, NapiForkPoint, NapiMergeRecord, NapiCompactionState, NapiTokenUsage, NapiStoredMessage, NapiHistoryEntry, NapiAppendResult, NapiCherryPickResult)
- Field order inside each interface is preserved exactly (napi-rs reads source order)

Pre-card SHA-256 of `codelet/napi/index.d.ts` must be captured before any source change and compared to the post-card regenerated file.

### F. Codelet-core dependency check

`Grep('codelet_core', codelet/napi/Cargo.toml)` returns the existing `codelet-core = { path = "../core" }` dependency added by RPC-031..RPC-034. **No new dependency needed.**

`Grep('tempfile', codelet/core/Cargo.toml)` confirms `tempfile = { workspace = true }` is already in codelet-core's `[dev-dependencies]` from the RPC-032/033/034 lifted-test integration tests. **No new dev-dependency needed** for the relocated test files. `lazy_static = { workspace = true }` is also present.

### G. Forbidden-arrow regression baseline

`codelet/rpc-embedded/tests/rpc_006_source_shape.rs` is the canonical guard against `rpc → napi` references. After RPC-035, the file must still pass and no new `use codelet_napi::...` line appears in any non-codelet-napi crate.

## Test count snapshot (must be preserved)

```bash
$ grep -c "^#\[test\]" codelet/napi/src/persistence/tests.rs
48
$ grep -c "^#\[test\]" codelet/napi/src/persistence/lazy_init_tests.rs
9
```

After the relocation:

```bash
$ grep -c "^#\[test\]" codelet/core/src/persistence/tests.rs
48
$ grep -c "^#\[test\]" codelet/core/src/persistence/lazy_init_tests.rs
9
```

And the existing NAPI-bridge integration tests stay where they are:

```bash
$ grep -c "^#\[test\]" codelet/napi/tests/session_persistence_test.rs
~23
$ grep -c "^#\[test\]" codelet/napi/tests/subordinate_session_persistence_test.rs
~4
```

## Caller-rewrite blueprint (mechanical edits only)

| Site | Before | After |
|---|---|---|
| `napi_bindings.rs:5` | `use super::*;` | `use codelet_core::persistence::{ create_session, create_session_with_provider, load_session, resume_last_session, list_sessions, delete_session, rename_session, fork_session, merge_messages, cherry_pick, append_message, append_message_with_metadata, get_message, get_session_messages, get_session_messages_full, update_session_tokens, set_session_tokens, set_compaction_state, clear_compaction_state, cleanup_orphaned_messages, store_blob, get_blob, blob_exists, process_envelope_for_blob_storage, rehydrate_envelope_blobs, MessageEnvelope, MessagePayload, UserContent, AssistantContent, SessionManifest, ForkPoint, MergeRecord, CompactionState, TokenUsage, StoredMessage, HistoryEntry, history, };` |
| `napi_bindings.rs:583` | `use super::{process_envelope_for_blob_storage as process_envelope_impl, rehydrate_envelope_blobs as rehydrate_envelope_impl};` | **delete** — the aliasing is no longer needed once the top-level explicit import is in place. Replace `process_envelope_impl` / `rehydrate_envelope_impl` call sites with `process_envelope_for_blob_storage` / `rehydrate_envelope_blobs`. |
| `napi_bindings.rs:19` (body of `persistence_set_data_directory`) | `set_data_directory(PathBuf::from(dir)).map_err(Error::from_reason)` | Inline the body: `let path = PathBuf::from(dir); codelet_common::set_data_directory(path).map_err(Error::from_reason)?; codelet_core::persistence::reset_stores_for_tests(); crate::credentials::reset_credential_store(); crate::graph::reset_graph_db(); Ok(())` |
| `napi_bindings.rs:25` (body of `persistence_get_data_directory`) | `get_data_dir().map(...).map_err(...)` | `codelet_common::get_data_dir().map(|p| p.to_string_lossy().to_string()).map_err(Error::from_reason)` |
| `napi_bindings.rs:268` (body of `persistence_add_history`) | `add_history_entry(entry)` | `codelet_core::persistence::history::add(entry).map_err(Error::from_reason)` |
| `napi_bindings.rs:278` (body of `persistence_get_history`) | `get_history(project_path.as_deref(), limit.map(|l| l as usize))` | `codelet_core::persistence::history::get(project_path.as_deref(), limit.map(|l| l as usize))` |
| `napi_bindings.rs:290` (body of `persistence_search_history`) | `search_history(&query, project_path.as_deref())` | `codelet_core::persistence::history::search(&query, project_path.as_deref())` |
| All `super::MessagePayload` / `super::UserContent` / `super::AssistantContent` refs (lines 863–930) | `super::MessagePayload` etc. | unqualified `MessagePayload` etc. (resolved via the new explicit import) |
| `mod.rs` | 115 lines (six `pub fn` declarations + history doc-comment) | ~15 lines: doc-comment + `pub use codelet_core::persistence::*;` + `#[cfg(not(feature = "noop"))] mod napi_bindings;` + `#[cfg(not(feature = "noop"))] pub use napi_bindings::*;` |
| `test_support.rs:21` | `crate::persistence::set_data_directory(temp_dir.path().to_path_buf()).expect(...)` | 4 inline lines: `codelet_common::set_data_directory(temp_dir.path().to_path_buf()).expect(...)` + `codelet_core::persistence::reset_stores_for_tests()` + `crate::credentials::reset_credential_store()` + `crate::graph::reset_graph_db()` |
| `tests.rs` → moved to `codelet/core/src/persistence/tests.rs` | `use super::*;` + `crate::persistence::set_data_directory` | `use crate::persistence::*;` + `codelet_common::set_data_directory` + `reset_stores_for_tests()` |
| `lazy_init_tests.rs` → moved to `codelet/core/src/persistence/lazy_init_tests.rs` | `use super::*;` + `use super::tests::TEST_MUTEX;` | `use crate::persistence::*;` + `use super::tests::TEST_MUTEX;` (no path change because both files are siblings under `crate::persistence`) |

## Conclusions

1. The cleanup is **purely mechanical** — no semantic changes to behaviour, only relocations and inlined-helper rewrites.
2. The 35 `#[napi]` shims keep their exact signatures; `index.d.ts` byte-identity is achievable by leaving the `Napi*` field orders untouched and not adding/removing any `#[napi] pub fn` items.
3. The 48 + 9 in-crate tests can be moved verbatim to codelet-core because their bodies already consume the lifted `codelet_core::persistence::*` symbols (the lift cards RPC-031..RPC-034 already routed them through the flat re-export).
4. The only non-test external caller of `crate::persistence::set_data_directory` is `test_support.rs:21`, so the inlined credentials/graph reset path in `persistence_set_data_directory` does not break any production call site.
5. The `ensure_directories` shim in `mod.rs` has zero remaining callers and can be deleted outright — downstream consumers can call `codelet_core::persistence::ensure_directories()` directly through the flat re-export.

## Reference commands

```bash
AstGrep(rust, "pub fn $NAME($$$ARGS) -> $RET { $$$BODY }", path=codelet/napi/src/persistence)
AstGrep(rust, "super::$NAME", path=codelet/napi/src/persistence/napi_bindings.rs)
Grep('^use super', codelet/napi/src/persistence, glob='*.rs', output_mode='content')
Grep('ensure_directories|set_data_directory|get_data_dir|add_history_entry|persistence::get_history|persistence::search_history', codelet/napi/src, glob='*.rs', output_mode='content')
```
