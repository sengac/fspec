# Review: RPC-041 — Replace GLOBAL_CHUNK_CALLBACK with tokio::broadcast sender

**Date:** 2026-05-21
**Reviewer:** Claude Code (fspec review skill)
**Status:** FIXED — moved back to implementing, all issues resolved, ready to re-advance

## Summary
- 🔴 Critical: 1 issue (now fixed)
- 🟡 Warnings: 5 issues (now fixed)
- 🟢 Observations: 1

---

## 🔴 Critical Issues (Now Fixed)

### 1. Rule [0] strict invariant violated by doc comments

**Rule [0]:** "After RPC-041 nothing in codelet/napi/src/session_manager.rs grep-matches the literal token `GLOBAL_CHUNK_CALLBACK`"

**Problem:** Five doc/line comments in `codelet/napi/src/session_manager.rs` still contained the literal token `GLOBAL_CHUNK_CALLBACK`:
- Line 73: `/// `GLOBAL_CHUNK_CALLBACK` static and its unsafe Send/Sync impls.`
- Line 87: `/// `GLOBAL_CHUNK_CALLBACK.get().is_none()`. Returns true once`
- Line 4292: `/// replaces the old `GLOBAL_CHUNK_CALLBACK` static.`
- Line 4498: `// previously dispatched through the deleted GLOBAL_CHUNK_CALLBACK).`
- Line 6685: `// poller, GLOBAL_CHUNK_CALLBACK, LoopStore) that stay in this file.`

The shape test (`scenario_global_chunk_callback_static_struct_and_unsafe_impls_are_removed`) strips line comments before checking, so the test passed — but the strict literal reading of rule [0] AND the Gherkin step "Then I find zero matches in the file" both require ZERO matches, comments included.

**Fix Applied:** Rewrote all five comments to refer to the artifact by description (e.g. "the legacy chunk-callback OnceCell static") rather than by literal token. `grep -n "GLOBAL_CHUNK_CALLBACK" codelet/napi/src/session_manager.rs` now returns zero matches. Build + all shape tests still pass.

---

## 🟡 Warnings (Now Fixed)

### 2. Coverage line drift — GLOBAL_CHUNK_CALLBACK removal scenario

**Problem:** Coverage reported test lines `1052-1089` for `scenario_global_chunk_callback_static_struct_and_unsafe_impls_are_removed`, but the actual function span is **1060-1099** (8-line drift, exactly the class of line-drift bug the card's Architecture note [2] warned against).

**Fix Applied:** Unlinked the stale mapping and re-linked with the correct range `1060-1099`, impl `codelet/napi/src/session_manager.rs:70-96`.

### 3. Coverage gap — SessionManager wires its senders into BackgroundSession

**Problem:** Impl coverage range `527-636` only covered `create_session_with_id` (which contains the first `BackgroundSession::new(... chunks_tx.clone(), status_changes_tx.clone())` call site at lines 552-553). The SECOND call site inside `create_isolated_session_with_id` at lines 798-799 was NOT covered.

**Fix Applied:** Replaced single mapping with two impl-file mappings covering both functions: `411-639` (create_session_with_id) and `642-884` (create_isolated_session_with_id).

### 4. Coverage gap — FspecHandler and command_emitter gates only point to helper

**Problem:** Impl coverage `82-95` for "The FspecHandler and bridge command_emitter gates consult a new is_global_chunk_callback_registered helper" only covered the helper definition. The two ACTUAL gate sites (`if !is_global_chunk_callback_registered()`) at lines 2995-3006 (FspecHandler) and 3275-3293 (command_emitter) were missing.

**Fix Applied:** Added two additional impl-file mappings for the gate sites at 2995-3006 and 3275-3293.

### 5. Coverage line drift — handle_output / set_status / BackgroundSession::new

**Problem:** Multiple impl line ranges for the BackgroundSession scenarios had drift:
- `handle_output` scenario: reported `757-820`, actual function `775-812`
- `set_status` scenario: reported `732-762`, actual function `741-770`
- `BackgroundSession::new` scenario: reported `432-510`, actual function `435-529`

**Fix Applied:** Re-linked each scenario with the precise function line range. Also added secondary impl mappings to the BackgroundSession struct field declarations (`326-341`) that document the new `chunks_tx` / `status_changes_tx` fields.

### 6. Coverage line drift — test ranges off-by-N at end

**Problem:** Several test line ranges over-shot the actual `}` closing brace by 4-6 lines, picking up the trailing `// ===` separator comment block.

**Fix Applied:** Each test range now starts at the `#[test]` attribute line and ends at the function's closing `}` line.

---

## 🟢 Observations

### 7. Internal spec inconsistency between rules [6] and [7]

Rule [6] specifies `OnceCell<parking_lot::Mutex<Option<...>>>` for `CHUNK_FANOUT_TSFN`, but rule [7c] specifies the helper body `CHUNK_FANOUT_TSFN.get().and_then(|m| m.lock().ok())...` — the `.lock().ok()` pattern requires `std::sync::Mutex` (fallible `LockResult`), not `parking_lot::Mutex` (infallible `lock()`).

The implementation chose `std::sync::Mutex`, which is consistent with rule [7c] and with the helper body that the test asserts. This is a documentation-side inconsistency in the rules, not an implementation bug. No fix required.

---

## Coverage Verification (Post-Fix)

### `replace-global-chunk-callback-background-session.feature`
- ✅ 4/4 scenarios fully covered
- All test line ranges match actual `#[test] fn ...` spans
- Impl ranges point to the precise functions they assert against

### `replace-global-chunk-callback-napi-shell.feature`
- ✅ 8/8 scenarios fully covered
- "SessionManager wires its senders" now covers BOTH `create_session_with_id` AND `create_isolated_session_with_id`
- "FspecHandler/command_emitter gates" now covers helper definition AND both gate sites
- "emit_block_notification_to_tui / spawn_footer_poller" covers both emit functions

---

## Files Reviewed

**Feature files:**
- `spec/features/replace-global-chunk-callback-background-session.feature`
- `spec/features/replace-global-chunk-callback-napi-shell.feature`

**Test files:**
- `codelet/sessions/tests/background_session_shape.rs`
- `codelet/sessions/tests/session_manager_shape.rs`
- `codelet/napi/tests/global_chunk_callback_napi_test.rs`

**Implementation files:**
- `codelet/napi/src/session_manager.rs` (comments cleaned up)
- `codelet/sessions/src/background_session.rs`
- `codelet/sessions/src/session_manager.rs`

**Coverage files:**
- `spec/features/replace-global-chunk-callback-background-session.feature.coverage`
- `spec/features/replace-global-chunk-callback-napi-shell.feature.coverage`

---

## Verification

- `cargo build -p codelet-napi` ✅ succeeds (after comment fixes)
- `cargo test -p codelet-sessions --tests` ✅ all 50 tests pass (9 unit + 14 background_session_shape + 20 session_manager_shape + 6 skeleton_invariants + 1 smoke)
- `cargo test -p codelet-napi --test global_chunk_callback_napi_test` ✅ all 18 tests pass
- `grep "GLOBAL_CHUNK_CALLBACK" codelet/napi/src/session_manager.rs` ✅ zero matches (rule [0] strictly satisfied)
- Feature files validate clean
- Coverage is 100% for both feature files with accurate line ranges
