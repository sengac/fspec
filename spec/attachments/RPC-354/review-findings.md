# Epic Review: RPC-354 — File Changes view in Rust TUI (port ChangedFilesViewer)

**Date:** 2026-06-26
**Reviewer:** Claude Code (fspec review skill) — 3 parallel subordinate reviewers
**Work Units Reviewed:** 3 (RPC-354 parent, RPC-355 backend, RPC-356 UI)

## Summary
- 🔴 Critical: 0 (within scope of these cards)
- 🟡 Warnings: addressed in-scope items below; 1 pre-existing broken test split out to a separate BUG card
- 🟢 Observations: feature is wired end-to-end, reachable from the board F-key, 100% scenario coverage on all three feature files, all new files <300 lines, no unwrap/expect/panic/todo in production paths.

## Work Unit Results

### RPC-355: Expose git changed-file status and per-file diff to the TUI transport — WARN
- 🟡 **Stale unrelated test `bug146_napi_attribute_scoping`** — its `assert_eq!(added_bug146.len(), 34)` diffs `codelet/rpc-types/src/lib.rs` vs HEAD and expects exactly the 34 BUG-146 `serde(rename)` additions to appear as *uncommitted* working-tree changes. BUG-146 is long committed (HEAD already has 36 such lines), so on a CLEAN tree this diff is 0 and the test fails (0 ≠ 34). RPC-355 added one new `#[serde(rename = "changeType")]` field, changing the failure number from 0 to 1. **This is a pre-existing broken test, not a regression introduced by the File Changes feature** (it is red on a clean checkout regardless). → Split out to a dedicated BUG card; NOT folded into RPC-355 to avoid scope creep into unrelated regression-guard tooling.
- 🟡 **Binary-file diff sentinel untested** — design doc lists `"[Binary file - no diff available]"` as a rule but RPC-355 never added a scenario/test for it. → FIX: add a transport-level test (in scope, restores design-doc traceability).
- 🟢 Change-type A/M/D derivation correct; reuses codelet/git (no reimplemented git logic); embedded + websocket Disconnected-guard parity confirmed; no-cwd returns empty/None.

### RPC-356: Dual-pane ChangedFilesView with F-key board wiring and Navigator integration — PASS
- 🟡 **Diff-pane scroll lacks a viewport-height bottom clamp** (`changed_files/mod.rs` `apply_diff_scroll`) — clamps to `diff_lines.len()-1` rather than using `ensure_visible`/page height like the file pane, allowing single-line overshoot. Arch note overstates `ensure_visible` reuse for the diff pane. → FIX: clamp the diff scroll to the viewport.
- 🟡 **`@wip` tag remains on `rust-changed-files-view.feature` despite done status.** → FIX: swap `@wip`→`@done`.
- 🟢 `color_of` test helper matches the first glyph on screen (theoretically fragile); module split + WheelVelocity reuse are clean; Given/When/Then ordering correct in all 8 scenarios.

### RPC-354: File Changes view (umbrella) — PASS
- 🟡 **`@step` import-anchor hack** `let _ = Action::OpenChangedFilesView;` in `tests/file_changes_view_rpc354.rs`. → FIX: remove the dummy binding (use a normal import / `#[allow(unused_imports)]` if needed).
- 🟡 **Compound `And` step** at `rust-file-changes-view.feature:54` folds an action+outcome into one `And` after `Then`. → FIX: split into `When ... / Then ...`, update the test `@step` comments to match.
- 🟡 **`@wip` tag remains on `rust-file-changes-view.feature` despite done status.** → FIX: swap `@wip`→`@done`.
- 🟢 Umbrella integrity confirmed: RPC-354 adds NO production code of its own (zero `RPC-354` matches in src); integration test drives the real stack; example map clean (3 rules → 3 scenarios → 3 examples, no open questions).

## Fix Results (Phase 5)

### RPC-355
- 🟡 Binary-file diff sentinel untested → ✅ Fixed: added scenario "file_diff returns the binary-file sentinel for a binary file" + test (`seed_repo_one_modified_binary`, asserts `Some("[Binary file - no diff available]")`). Coverage 100% (5/5). Also re-linked the 4 pre-existing scenarios whose line ranges had shifted when the new test was inserted.
- 🟡 Stale `bug146_napi_attribute_scoping` → ⏭️ Out of scope: pre-existing broken test, tracked as **BUG-147** (not touched).

### RPC-356
- 🟡 Diff-pane scroll overshoot → ✅ Fixed: `apply_diff_scroll` now clamps via `max_diff_scroll() = diff_lines.len().saturating_sub(viewport_height)`. New scenario "Diff pane scroll stops at the last full page" + test. mod.rs = 296 lines. Coverage 100% (9/9).
- 🟡 `@wip` on feature → ✅ Fixed: swapped `@wip`→`@done`.

### RPC-354
- 🟡 Compound `And` step → ✅ Fixed: split into `When ... / Then ...`; test `@step` comments updated verbatim. Coverage 100% (3/3).
- 🟡 Import-anchor hack → ✅ Fixed: removed dummy binding, dropped unused `Action` import.
- 🟡 `@wip` on feature → ✅ Fixed: swapped `@wip`→`@done`.

## Final Verification
- `cargo test -p codelet-fspec-tui` — ✅ all green
- `cargo test -p codelet-git` — ✅ all green
- `cargo test -p codelet-rpc-types` — only `bug146_napi_attribute_scoping` fails (pre-existing, BUG-147); the only failure workspace-wide
- Feature files valid: ✅ (rust-file-changes-view, rust-changed-files-view, git-changed-files-transport, git-change-type-derivation)
- Coverage complete: ✅ 100% on all four features
- All three cards marked `done`.

## Fix Plan (Phase 4 — sequential, via worker, ACDD)
1. **New BUG card** for the stale `bug146_napi_attribute_scoping` test (separate, tracked; not part of this epic's scope).
2. **RPC-355** → reopen to implementing: add binary-sentinel transport test; re-validate; done.
3. **RPC-356** → reopen to implementing: diff-pane bottom-clamp fix + test; `@wip`→`@done`; re-validate; done.
4. **RPC-354** → reopen to implementing: split the compound `And` step + fix test `@step` text; remove the import-anchor hack; `@wip`→`@done`; re-validate; done.
