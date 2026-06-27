# Epic Review: checkpoints-view-rust (RPC-361)

**Date:** 2026-06-27
**Reviewer:** Claude Code (fspec review skill) — 5 parallel subordinate reviewers
**Work Units Reviewed:** 5 children (RPC-362..366)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 7 (1 broken coverage mapping + coverage-range drift + 2 missing dispatch tests + cosmetics)
- Statuses: RPC-362 PASS · RPC-363 WARN · RPC-364 PASS · RPC-365 PASS · RPC-366 PASS
- Whole crate: `cargo test -p codelet-fspec-tui --lib` 302/0; clippy clean; all checkpoint files < 300 lines; no production unwrap/expect/panic.

## Findings & fix plan (review order)

### RPC-362 — Checkpoint transport — PASS
- 🟡 W1: Degraded no-index path makes the "200 most recent" cap non-deterministic (all timestamps fall back to now). Add a comment documenting the ordering contract.
- 🟡 W2: `delete_all` swallows index-dir removal error (`let _ = remove_dir_all`). Acceptable (ref deletion is source of truth) but add a clarifying comment.
- 🟢 `restore_checkpoint_file` and `delete_all_checkpoints` have no direct integration test → ADD a `restore_file` and a `delete_all` happy-path test to close coverage on all 7 helpers.

### RPC-363 — Shared diff/row module — WARN (has a real fix)
- 🟡 W1 (FIX): Stale coverage links in the PARENT feature `rust-changed-files-view.feature`. Two scenarios still point at the DELETED `changed_files/row.rs` and `changed_files/diff_render.rs`. Re-link to the moved `views/diff_common/row.rs` and `views/diff_common/diff_render.rs`.

### RPC-364 — Three-pane view — PASS
- 🟡 W1 (FIX): Coverage for "Opening flips the Navigator…" links impl `navigator.rs:139-148` (ModelSelector/ChangedFiles arms) instead of the actual Checkpoints arms at `navigator.rs:153-156`. Re-link.
- 🟡 W2: The Tab green-bg highlight test asserts `any_green_bg` globally; tighten to assert the bg on the "Files" header row.

### RPC-365 — Restore actions — PASS
- 🟡 W1 (FIX): `restore_checkpoint_all` has no dispatch integration test (only the single-file path is asserted at the dispatch boundary). Add a `RestoreCheckpointAll` → `restore_checkpoint_all` MockBackend dispatch test.
- 🟡 W2: `@step` ordering in `checkpoint_restore_dispatch_rpc365.rs` inverts Then/And vs the feature; reorder for living-doc fidelity (text already matches).

### RPC-366 — Delete actions — PASS
- 🟢 O1 (FIX): Coverage test ranges for `checkpoint-delete-dispatch` overshoot EOF (`80-104` vs actual `74-97`). Re-link to the real test bodies.
- Typed-confirm gating fully verified (wrong-phrase → no dispatch; exact `DELETE ALL` → dispatch). Solid.

## Fix execution & results
All fixes applied by the implementation worker under supervisor control, independently verified by the supervisor.

- RPC-363 W1 → ✅ Re-pointed the 2 stale `rust-changed-files-view.feature` coverage links to `views/diff_common/row.rs:50-67` and `views/diff_common/diff_render.rs:40-50`. Coverage warnings 2 → **0**; still 18/18.
- RPC-364 W1 → ✅ Re-linked the Navigator-flip scenario to `navigator.rs:153-156`. W2 → ✅ Tightened the Tab green-bg test to assert the bg on the "Files" header row specifically.
- RPC-365 W1 → ✅ Added `dispatching_restore_checkpoint_all_calls_the_transport` dispatch test + scenario (checkpoint-restore-dispatch now 2/2). W2 → ✅ Reordered `@step` comments to match Gherkin Then/And order.
- RPC-366 O1 → ✅ Re-linked both checkpoint-delete-dispatch scenarios to real test bodies (`41-71`, `74-97`).
- RPC-362 → ✅ Added `restore_checkpoint_file` + `delete_all_checkpoints` integration tests (checkpoint-transport 6/8 → **8/8**) + documenting comments on degraded ordering and best-effort index removal.

## Final verification (supervisor-run)
- `cargo test -p codelet-fspec-tui --lib`: **302 passed, 0 failed**
- `cargo test -p codelet-rpc --test checkpoint_transport_rpc362`: **8 passed**
- restore-dispatch: **2 passed** · delete-dispatch: **2 passed**
- clippy clean; all checkpoint/diff_common/rpc files < 300 lines
- `rust-changed-files-view` coverage: 18/18, **no file-not-found / stale links**
- All five children advanced to **done**.
