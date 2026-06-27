# Review Findings — RPC-368 & RPC-369 (Click-to-select in TUI views)

**Date:** 2026-06-27
**Reviewer:** Claude Code (fspec review-skill, parallel reviewers)
**Work Units Reviewed:** 2 (RPC-368, RPC-369)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 3 (RPC-368 ×1, RPC-369 ×2)
- 🟢 Observations: 5

Both work units PASSED review. Tests green, builds clean, clippy clean, coverage
100%, end-to-end wiring confirmed, all Gherkin steps have exactly-matching `// @step`
comments, all production files under the 300-line ceiling.

---

## RPC-368 — Click a file row to select it in the Changed Files view — PASS

### 🟡 Warnings
1. `codelet/fspec-tui/src/views/changed_files/mod.rs` is **exactly at the 300-line
   ceiling**. The Down-arm added by this WU is small, but the file now has zero
   headroom. → FIX: refactor for headroom.

### 🟢 Observations
1. Shared `tests.rs` file header references the parent feature (RPC-356); the RPC-368
   tests carry their own section banner referencing
   `changed-files-view-click-to-select.feature` — traceability preserved at section
   level. Acceptable.
2. No-op path returns `Consumed` (via `move_selection`), test asserts "not Emit"
   rather than `Consumed` specifically. Adequate.
3. `mouse.rs` is clean — no unwrap/expect/todo; safe `saturating_sub`/`match` handling.

### Fix decision
- **W1**: Move the mouse-handling code out of `mod.rs` into the existing
  `changed_files/mouse.rs` to drop `mod.rs` comfortably under 300 lines.

---

## RPC-369 — Click a checkpoint or file row to select it in the Checkpoints view — PASS

### 🟡 Warnings
1. **Architecture-note drift.** Note [0] describes inline index math in `handle_click`,
   but the implementation extracted the row→index mapping into a `row_target` helper
   (`mod.rs:273-289`). The code is *better* than the note; the note was not updated.
   → FIX: add/clarify an architecture note documenting `row_target`.
2. **Duplicated boundary expression** `offset >= len.saturating_sub(scroll)` lives in
   both `changed_files/mouse.rs:30` and `checkpoints/mod.rs:285`. Not shareable today
   (distinct `Pane` enums + field sets per view). → ACCEPTED; documented for a future
   consolidation. No code change.

### 🟢 Observations
1. Rule [5] second clause ("click empty space below last row / outside rects") is not
   directly tested at the click level for the Checkpoints view (covered transitively by
   `row_target` returning `None`). → FIX: add an explicit no-op test for completeness.
2. `handle_click` returns `Ignored` for outside-pane vs `Consumed` for in-pane-no-row;
   consistent with intent, worth a one-line comment.
3. `left_click` helper dispatches through real `handle_event` → `handle_mouse` (not
   calling `handle_click` directly), exercising the dialog-guard + Down-arm ordering
   end-to-end. Good.

### Fix decision
- **W1**: Update the architecture note to reflect the `row_target` helper.
- **Obs 1**: Add an explicit "click below last checkpoint row is a no-op" scenario +
  test (improves rule [5] coverage).
- **W2**: Accepted as-is (documented above).

---

## Fix Results
_(updated after fixes applied — see below)_

### RPC-368
- 🟡 W1 (mod.rs at 300-line ceiling) → ✅ Fixed: moved `handle_mouse` (Down arm + wheel
  handling) from `mod.rs` into the sibling `changed_files/mouse.rs`. **mod.rs 300 → 276**,
  mouse.rs 39 → 69. Coverage impl ranges re-linked to the new `mouse.rs:44-68`. Pure
  refactor, no behavior change. `cargo test --lib changed_files` 26/26 green.

### RPC-369
- 🟡 W1 (architecture-note drift re: `row_target`) → ✅ Fixed: added an architecture note
  documenting the extracted `row_target(row, list_len) -> Option<usize>` helper.
- 🟡 W2 (duplicated boundary expression across the two views) → ✅ Accepted as-is: the two
  views have distinct `Pane` enums and field sets, so no shared abstraction today;
  documented for a future consolidation.
- 🟢 Obs 1 (missing explicit "click below last row" test for Checkpoints) → ✅ Fixed: added
  scenario "Clicking empty space below the last checkpoint row changes nothing" + a
  meaningful test (a naive clamp-to-last impl would fail it). Feature now 6 scenarios.
- Coverage-link accuracy regression (introduced when the new test was inserted mid-file,
  shifting later tests' line numbers) → ✅ Fixed: re-linked all 6 checkpoint scenarios to
  their correct current test ranges (verified line-by-line). `audit-coverage` all valid.

## Final Verification
- Full crate `cargo test`: ✅ **319 + all suites pass, 0 failed**
- `cargo build`: ✅ clean
- `cargo clippy --lib`: ✅ no warnings
- Coverage: ✅ changed-files 5/5 (100%), checkpoints 6/6 (100%); audit-coverage all valid
- Feature files valid: ✅ both
- Production file line counts: ✅ changed_files/mod.rs 276, mouse.rs 69, checkpoints/keys.rs
  112, checkpoints/mod.rs 296 — all < 300
- Both work units: ✅ **done**

| Work Unit | Title | Status | Issues |
|---|---|---|---|
| RPC-368 | Click a file row to select it (Changed Files view) | ✅ PASS | 1 🟡 fixed |
| RPC-369 | Click a checkpoint/file row to select it (Checkpoints view) | ✅ PASS | 2 🟡 + 1 🟢 fixed + coverage re-link |
