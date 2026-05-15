# Review: RPC-017 — Priority reorder persistence: wire [ / ] to backend

**Date:** 2026-05-15
**Reviewer:** Claude Code (fspec review-skill)
**Reviewer scope:** Single card review (not epic-wide) — RPC-017 only
**Card status at review:** done

---

## Status: ✅ PASS

No critical or fix-worthy issues found. All ACDD invariants hold, all scenarios are 100% covered, every test passes, the build is clean (the lone preexisting `ambiguous glob re-exports` warning in `codelet-napi` is unrelated to RPC-017), and the architecture matches the work-unit's stated plan.

This review was performed independently of the prior `review-findings.md` content (which has been overwritten).

---

## 🔴 Critical Issues (Must Fix)

None.

---

## 🟡 Warnings (Should Fix)

None.

---

## 🟢 Observations (Nice to Have — NOT fixed, deferred to a future card if desired)

1. **Cross-transport-parity When-step wording vs test invocation (cosmetic).**
   Three scenarios in `rpc017-cross-transport-parity.feature` use When-steps phrased as `client.move_work_unit_up(context::current(), "...")` (FspecService client level), but the corresponding tests call `backend.move_work_unit_up(...)` on the `FspecBackend` trait. The behavior IS exercised — `EmbeddedFspecBackend::move_work_unit_up` is literally a one-line `self.client.move_work_unit_up(context::current(), id).await?` delegate, so the test transitively asserts what the scenario describes. The mismatch is purely cosmetic wording. Not worth fixing in this card (no scope creep) but a future card could either reword the When-step to `backend.move_work_unit_up(...)` or replace the test bodies with raw `client.move_work_unit_up(context::current(), ...)` calls for literal fidelity.

2. **Architecture note `[4]` references the wrong feature file (informational).**
   The note says the build_service regression is "covered by a new scenario in rpc017-cross-transport-parity.feature". The scenario was actually split out into its own feature `rpc017-build-service-cwd-attachment.feature` (per the docstring referencing VAL-005's 1:1 feature-to-test-file rule). The behavior IS covered — just in the sibling feature — so this is a historical-doc staleness only, not a coverage gap.

---

## Coverage Verification

All five RPC-017 feature files report 100% scenario coverage:

| Feature file | Scenarios | Coverage | Tests pass |
|---|---|---|---|
| `rpc017-work-units-write-helper.feature` | 8 | 100% (8/8) | ✅ 8/8 |
| `rpc017-cross-transport-parity.feature` | 7 | 100% (7/7) | ✅ 7/7 |
| `rpc017-app-dispatch-reorder.feature` | 4 | 100% (4/4) | ✅ 4/4 |
| `rpc017-source-shape.feature` | 9 | 100% (9/9) | ✅ 9/9 |
| `rpc017-build-service-cwd-attachment.feature` | 1 | 100% (1/1) | ✅ 1/1 |
| **Total** | **29** | **100%** | **29/29 pass** |

Test files (each verified to exist with `@step` comments matching the Gherkin step text):
- `codelet/core/tests/work_units_write_test.rs`
- `codelet/fspec-tui/tests/move_work_unit_rpc017.rs`
- `codelet/fspec-tui/tests/app_dispatch_reorder_rpc017.rs`
- `codelet/fspec-tui/tests/source_shape_rpc017.rs`
- `codelet/fspec/src/common.rs` (inline `#[cfg(test)] mod tests`)

Implementation files exercised by the tests:
- `codelet/core/src/work_units_write.rs` (new, 185 LoC)
- `codelet/common/src/file_lock.rs` (new, 196 LoC)
- `codelet/rpc/src/lib.rs` (FspecService trait + FspecServiceImpl additions)
- `codelet/fspec-tui/src/transport/mod.rs` (FspecBackend trait additions)
- `codelet/fspec-tui/src/transport/embedded.rs` (Embedded impl)
- `codelet/fspec-tui/src/transport/websocket.rs` (WebSocket impl)
- `codelet/fspec-tui/src/app/dispatch.rs` (Action handlers)
- `codelet/fspec-tui/src/store/board.rs` (re-anchor selection by id)
- `codelet/napi/src/work_units_watcher.rs` (NAPI exports)
- `codelet/napi/src/schedule_handler.rs` (refactored to use `with_file_lock`)
- `codelet/fspec/src/common.rs` (`build_service` cwd attachment + regression test)

---

## ACDD Compliance Checks

### A. Feature File Compliance — ✅
- All five feature files validate via `fspec validate`.
- Every scenario has correct Given/When/Then ordering.
- No placeholder text (`[role]`, `[action]`, `[benefit]`).
- Each feature carries an architecture doc-string explaining its slice.
- `@RPC-017` tag is present on all five features.

### B. Example Map Alignment — ✅
- 9 rules in the example map are all reflected in scenarios across the five files.
- 6 examples in the map all map to scenarios (e.g. example [0] "3rd unit moves up + persists" → `move_work_unit_up_swaps_with_predecessor`; example [3] "concurrent TS + Rust writers cooperate" → `concurrent_move_work_unit_calls_serialize_via_lock`).
- No unanswered red-card questions.
- Architecture notes [0]–[4] all match the implementation:
  - [0] `with_file_lock` lifted into `codelet/common/src/file_lock.rs`, schedule_handler refactored.
  - [1] `Action::ReorderUp/Down` spawn fire-and-forget tasks; no store mutation in spawned task.
  - [2] New module `codelet/core/src/work_units_write.rs` keeps work_units.rs read-focused; new file is 185 LoC (< 300 ceiling).
  - [3] `SharedFspecService::cwd()` delegation; missing cwd returns Err (covered by scenario `FspecService::move_work_unit_up returns Err when no cwd is attached`).
  - [4] `build_service` chains `.with_cwd(workspace.to_path_buf())`; covered by inline test in `common.rs`. (See observation 2 about the note's slightly-stale feature-file reference.)

### C. Test Coverage Compliance — ✅
- Every Gherkin scenario has a corresponding test method.
- Every test carries `@step` comments matching the Gherkin step text exactly (spot-checked across all five test files).
- Tests verify actual behavior — they read/write real temp workspaces (`tempfile::TempDir`), drive real tarpc transports (`bind_and_serve`), spawn concurrent threads to exercise the inter-process lock, and assert on post-state JSON content. No trivial `assert!(true)` patterns.
- `fspec show-coverage` confirms all scenarios linked to test files + line ranges + implementation files + line ranges.

### D. Implementation Quality — ✅
- **SRP**: `move_work_unit` lives in its own `work_units_write.rs` sibling — read-side stays in `work_units.rs`. Lock helper isolated in `codelet/common/src/file_lock.rs`. Clean separation.
- **DRY**: The proper-lockfile-compatible mkdir lock previously inlined in `schedule_handler.rs` is now a single source of truth in `with_file_lock`. Both the new `work_units_write::move_work_unit` and the existing `schedule_handler` go through it.
- **No shortcuts / no half-written code**: No `todo!()`, `unimplemented!()`, `unwrap()` in production paths. Test code uses `unwrap()` under explicit `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **Wired end-to-end**: `[`/`]` keybindings in `views/board.rs:168-173` emit `Action::ReorderUp/Down` → `App::dispatch` (lines 190-216) spawns fire-and-forget task calling `backend.move_work_unit_up/_down` → `EmbeddedFspecBackend`/`WebSocketFspecBackend` forward through the same tarpc `FspecService` → `FspecServiceImpl::move_work_unit_up/_down` delegates to `codelet_core::work_units_write::move_work_unit`. Persistence triggers watcher → `Action::WorkUnitsLoaded` → `BoardStore::replace_work_units` re-anchors selection by id. Verified the full chain reads coherently.
- **Error handling**: All `fs::read_to_string`, `serde_json::from_str`, `fs::write`, `fs::rename` use `with_context`. Lock errors propagate as `Err(String)`. RPC handlers stringify with `format!("{e:#}")` so both transports surface identical diagnostics.
- **File-size discipline**: New RPC-017 files are under 300 LoC (work_units_write.rs=185, file_lock.rs=196). Preexisting files that already exceeded the ceiling (work_units.rs=413, rpc/lib.rs=500, common.rs=511, websocket.rs=431) were already over-limit before this card; RPC-017's contribution to each is minimal and architecture note [2] explicitly acknowledges this for `work_units.rs` and justifies the sibling-module split as the response.

### E. Build & Test Verification — ✅
- `cargo build --workspace` succeeds. (One preexisting `ambiguous glob re-exports` warning in `codelet-napi`; unrelated to RPC-017.)
- `cargo test --test work_units_write_test -p codelet-core` — 8/8 pass.
- `cargo test --test move_work_unit_rpc017 -p codelet-fspec-tui` — 7/7 pass.
- `cargo test --test app_dispatch_reorder_rpc017 -p codelet-fspec-tui` — 4/4 pass.
- `cargo test --test source_shape_rpc017 -p codelet-fspec-tui` — 9/9 pass.
- `cargo test -p codelet-fspec build_service_attaches_workspace_cwd` — 1/1 pass.

### F. Cross-Cutting Concerns — ✅
- **No shared-logic duplication**: The mkdir-lock dance is now centralised; no duplicate write protocols across `work_units_write.rs` and `schedule_handler.rs`.
- **Architecture-notes-to-code fidelity**: Implementation tracks the stated plan; the only stale reference is the file-file pointer in note [4] (see observation 2).
- **Security**: No unsanitized input; ids flow as `String` through tarpc. Lock-stale detection (10s) plus best-effort `rmdir` release prevent unbounded blockage from a crashed writer.
- **Performance**: Linear backoff capped at 500ms; max 10 retries; helper does atomic temp-file + rename so partial-write corruption is impossible. Concurrent test `concurrent_move_work_unit_calls_serialize_via_lock` verifies the lock actually serialises.
- **View isolation**: `source_shape_rpc017.rs` regression test enforces that `codelet/fspec-tui/src/views/*.rs` never imports `codelet_core::`, `codelet_napi::`, `tarpc::`, `tokio_tungstenite::`, nor constructs a tokio Runtime directly — passing.
- **Tag registry compliance**: All RPC-017 feature tags are registered (`@rust`, `@rpc`, `@tui`, `@persistence`, `@board-view`, `@work-units`, `@source-shape`, `@parity`, `@done`, `@RPC-017`).

---

## Fix Results

No fixes were applied because no fix-worthy issues were found. The two observations above are non-blocking and intentionally deferred (per the user's "no scope creep" directive).

---

## Final Verification

- All tests pass: ✅ (29/29 RPC-017 scenarios → 29/29 tests pass)
- Build succeeds: ✅ (workspace builds clean apart from preexisting napi glob-reexport warning)
- Coverage complete: ✅ (29/29 = 100% across five feature files)
- Feature files valid: ✅ (`fspec validate` clean on all five files)
- Tags valid: ✅ (all RPC-017 feature tags registered)

---

## Files Reviewed

### Feature files
- `spec/features/rpc017-work-units-write-helper.feature`
- `spec/features/rpc017-cross-transport-parity.feature`
- `spec/features/rpc017-app-dispatch-reorder.feature`
- `spec/features/rpc017-source-shape.feature`
- `spec/features/rpc017-build-service-cwd-attachment.feature`

### Test files
- `codelet/core/tests/work_units_write_test.rs` (8 tests)
- `codelet/fspec-tui/tests/move_work_unit_rpc017.rs` (7 tests)
- `codelet/fspec-tui/tests/app_dispatch_reorder_rpc017.rs` (4 tests)
- `codelet/fspec-tui/tests/source_shape_rpc017.rs` (9 tests)
- `codelet/fspec/src/common.rs` (inline `mod tests` — 1 RPC-017 test)

### Implementation files
- `codelet/core/src/work_units_write.rs` (NEW — 185 LoC)
- `codelet/common/src/file_lock.rs` (NEW — 196 LoC)
- `codelet/common/src/lib.rs` (declares `pub mod file_lock`)
- `codelet/core/src/lib.rs` (declares `pub mod work_units_write`)
- `codelet/rpc/src/lib.rs` (FspecService trait + FspecServiceImpl)
- `codelet/fspec-tui/src/transport/mod.rs` (FspecBackend trait)
- `codelet/fspec-tui/src/transport/embedded.rs`
- `codelet/fspec-tui/src/transport/websocket.rs`
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/store/board.rs`
- `codelet/fspec-tui/src/views/board.rs`
- `codelet/napi/src/work_units_watcher.rs`
- `codelet/napi/src/schedule_handler.rs`
- `codelet/fspec/src/common.rs`

### Attachments
- `spec/attachments/RPC-017/typescript-reference.md`
- `spec/attachments/RPC-017/ast-research-rpc017-integration-sites.md`
