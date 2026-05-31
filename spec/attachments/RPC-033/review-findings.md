# Epic Review: RPC-033 — Lift SessionStore manifest + load_session + append_message_with_metadata into codelet-core::persistence::manifest

**Date:** 2026-05-20
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-033 — no child work units)
**Scope:** Strictly RPC-033 acceptance criteria — no scope creep into RPC-025/031/032/034.

## Summary

- 🔴 Critical: 1 issue → **FIXED**
- 🟡 Warnings: 1 issue (7 compiler warnings) → **FIXED**
- 🟢 Observations: 3

## Work Unit Results

### RPC-033 — PASS (after fixes)

#### Files Reviewed

**Feature files**
- `spec/features/lifted-session-store-in-core-persistence.feature`
- `spec/features/napi-re-export-shim-for-session-store.feature`

**Test files**
- `codelet/core/tests/session_store_lifted_test.rs` (5 scenarios, 206 lines, all @step comments correct)
- `codelet/napi/tests/session_store_lift_shim_test.rs` (3 scenarios + 1 compile-time identity proof, 256 lines, all @step comments correct)

**Implementation files**
- `codelet/core/src/persistence/manifest.rs` (1084 lines — NEW lifted module)
- `codelet/core/src/persistence/mod.rs` (now declares `pub mod manifest;` + `pub use manifest::*;`)
- `codelet/core/src/persistence/sessions.rs` (collapsed to thin alias for RPC-026 backward compat)
- `codelet/napi/src/persistence/mod.rs` (now a 185-line facade with flat `pub use codelet_core::persistence::*;`)
- `codelet/napi/src/persistence/storage.rs` — DELETED ✅
- `codelet/napi/src/persistence/types.rs` — DELETED ✅
- `codelet/napi/src/persistence/lazy_init_tests.rs` (rewired to use codelet-core test-only accessors)

#### Rule-by-Rule Verification (all 10 acceptance criteria)

| # | Rule | Status |
|---|------|--------|
| 0 | All session types live in `codelet/core/src/persistence/manifest.rs` with identical serde derives, no `#[napi]` attributes | ✅ Verified at lines 43–193 of manifest.rs |
| 1 | All session-level free functions + MESSAGE_STORE/SESSION_STORE singletons live in codelet-core | ✅ All 26 functions present in manifest.rs lines 545–1083; singletons at lines 464–467 |
| 2 | `storage.rs` + `types.rs` deleted; flat re-export shim compiles | ✅ Files deleted, `pub use codelet_core::persistence::*;` at line 41 of napi/persistence/mod.rs |
| 3 | On-disk format byte-identical | ✅ Verified by `session_manifest_round_trips_via_sessions_json_from_core` |
| 4 | `SessionStore::new()` calls `codelet_common::get_data_dir()` directly | ✅ Line 219 of manifest.rs |
| 5 | RPC-026 double-delete removed; canonical delete in lifted facade | ✅ `delete_session` at manifest.rs:753; no double-delete in napi/mod.rs |
| 6 | `set_data_directory` in napi delegates to `reset_stores_for_tests` | ✅ napi/persistence/mod.rs:75 |
| 7 | BLOB_STORE stays in NAPI | ✅ napi/persistence/mod.rs:53–55 |
| 8 | `list_sessions` is `pub` in lifted location | ✅ manifest.rs:721 |
| 9 | Test-only accessors `is_message_store_initialized_for_tests` + `is_session_store_initialized_for_tests` exposed by core | ✅ manifest.rs:530–538 |

#### Test Execution Results

| Suite | Tests | Result |
|-------|-------|--------|
| `cargo test -p codelet-core --test session_store_lifted_test` | 5 | ✅ all pass |
| `cargo test -p codelet-napi --test session_store_lift_shim_test` | 3 (+ 1 compile-time identity check) | ✅ all pass |
| `cargo test -p codelet-napi --lib persistence::tests` | 48 | ✅ all pass |
| `cargo test -p codelet-napi --lib persistence::lazy_init_tests` | 9 | ✅ all pass (incl. BUG-122 invariants) |
| `cargo test -p codelet-napi --test session_persistence_test` | 23 | ✅ all pass |
| `cargo test -p codelet-napi --test subordinate_session_persistence_test` | 4 | ✅ all pass |
| **Total RPC-033-related tests** | **92** | ✅ **92/92 pass** |

`cargo clippy -p codelet-core --lib --no-deps` — clean.
`cargo clippy -p codelet-napi --lib --no-deps` — clean (after fixes).

#### Coverage

- `lifted-session-store-in-core-persistence.feature` — 100% (5/5 scenarios covered)
- `napi-re-export-shim-for-session-store.feature` — 100% (3/3 scenarios covered)

---

## 🔴 Critical Issues (Found and Fixed)

### 1. Placeholder tags in `napi-re-export-shim-for-session-store.feature`

**Finding:** Line 9 of the feature contained literal placeholder tags `@component` and `@feature-group` left over from the scenario-generation prefill. `fspec validate-tags` reported:

```
✗ spec/features/napi-re-export-shim-for-session-store.feature has tag violations:
  Placeholder tag: @component
  Placeholder tag: @feature-group
```

The companion feature already had `@rpc` + `@persistence` (component) and `@session-management` (feature-group), so the placeholders were never replaced with real values.

**Fix Applied:** Removed `@component` and `@feature-group` from line 9; kept `@critical`. The existing tags `@rpc`, `@persistence`, `@session-management`, `@napi`, `@rust`, `@refactor`, `@RPC-033` already satisfy all required categories.

**Verification:** `fspec validate-tags spec/features/napi-re-export-shim-for-session-store.feature` now reports `✓ All tags are registered`.

---

## 🟡 Warnings (Found and Fixed)

### 1. Compiler warnings — shadowed glob re-exports and unused imports

**Finding:** RPC-033's introduction of `pub use codelet_core::persistence::*;` in `codelet/napi/src/persistence/mod.rs` (line 41) shadowed the still-declared `mod history;` and `mod message_envelope;` thin shim modules that were left over from RPC-025 and RPC-031. `cargo build -p codelet-napi` emitted 7 warnings:

```
warning: private item shadows public glob re-export → mod history;
warning: private item shadows public glob re-export → mod message_envelope;
warning: unused import: codelet_core::persistence::history::HistoryStore
warning: unused import: codelet_core::persistence::HistoryEntry
warning: unused import: codelet_core::persistence::message_envelope::*
warning: unused import: history::*
warning: unused import: message_envelope::*
```

Architecture Note [2] of RPC-033 explicitly states the post-lift `mod.rs` must declare **only** "blob, blob_processing, napi_bindings, tests, lazy_init_tests" — so the lingering `history` and `message_envelope` shim mods are out of scope for the final RPC-033 layout. The warnings were introduced *by* RPC-033's flat re-export, so fixing them falls within this work unit.

**Fix Applied:**
- Deleted `codelet/napi/src/persistence/history.rs` — its `pub use codelet_core::persistence::history::HistoryStore;` and `pub use codelet_core::persistence::HistoryEntry;` re-exports are now provided directly by the RPC-033 flat re-export.
- Deleted `codelet/napi/src/persistence/message_envelope.rs` — its `pub use codelet_core::persistence::message_envelope::*;` is now provided directly by the flat re-export. Its single test (`test_blob_threshold`) was already a verbatim duplicate of `test_should_use_blob_storage` at `codelet/napi/src/persistence/blob.rs:179`, so no test coverage was lost.
- Removed `mod history;`, `mod message_envelope;`, `pub use history::*;`, `pub use message_envelope::*;` from `napi/src/persistence/mod.rs`.

**Verification:** `cargo build -p codelet-napi` and `cargo clippy -p codelet-napi --lib --no-deps` now both complete with zero warnings. All 92 RPC-033-related tests still pass.

---

## 🟢 Observations (Not Fixed — Out of Scope or Acceptable)

### 1. `manifest.rs` is 1084 lines

The project coding standard says "Keep files under 300 lines" — but this is a TypeScript guideline (it talks about Vite, ES modules, etc.). Rust convention is more relaxed and the lifted module intentionally co-locates `SessionStore` + every related free function for the RPC-033 lift. Splitting it would require a separate refactor work unit and was not requested in the acceptance criteria.

### 2. Rule [0] wording mentions `PastedContent` as a manifest type

Rule [0] lists "SessionManifest TokenUsage MergeRecord PastedContent ForkPoint CompactionState SessionLineage" as types that should live in `manifest.rs`. In reality, `PastedContent` is an enum belonging to history's `HistoryEntry` (RPC-025) and has nothing to do with session manifests. The implementation correctly leaves `PastedContent` in `codelet/core/src/persistence/history.rs`. This is a wording artifact in the example map rule, not a code defect — there is no NAPI `PastedContent` symbol on the session side to lift.

### 3. `set_session_tokens` has a `_output: u64` parameter that is ignored

`manifest.rs:990–1013` defines `set_session_tokens(session, input, _output, cache_read, cache_create, cumulative_input, cumulative_output)` where the `_output` argument is dropped in favour of `cumulative_output`. This signature is preserved verbatim from the pre-lift NAPI code — RPC-033 is a byte-identical lift, not an API redesign. Cleaning this up is out of scope.

---

## Fix Results

### RPC-033 — Lift SessionStore manifest

- 🔴 **Placeholder tags `@component` + `@feature-group` on `napi-re-export-shim-for-session-store.feature`** → ✅ **Fixed**: Removed via direct edit; `validate-tags` now passes.
- 🟡 **7 compiler warnings from shadowed glob re-exports** → ✅ **Fixed**: Deleted the redundant `napi/src/persistence/{history,message_envelope}.rs` shim files and removed their `mod`/`pub use` lines from `napi/src/persistence/mod.rs`. `cargo build -p codelet-napi` and `cargo clippy -p codelet-napi --lib --no-deps` now complete with zero warnings.

## Final Verification

- All 92 RPC-033-related tests pass: ✅
- `cargo build -p codelet-core --lib`: ✅
- `cargo build -p codelet-napi`: ✅ (zero warnings)
- `cargo clippy -p codelet-core --lib --no-deps`: ✅ (zero warnings)
- `cargo clippy -p codelet-napi --lib --no-deps`: ✅ (zero warnings)
- `fspec validate spec/features/lifted-session-store-in-core-persistence.feature`: ✅
- `fspec validate spec/features/napi-re-export-shim-for-session-store.feature`: ✅
- `fspec validate-tags` on both feature files: ✅
- `fspec show-coverage`: 100% on both features (5/5 + 3/3 scenarios) ✅

## Summary Table

| Work Unit | Title                                  | Status   | Issues       |
|-----------|----------------------------------------|----------|--------------|
| RPC-033   | Lift SessionStore manifest             | ✅ PASS  | 2 fixed (1 critical, 1 warning) |
