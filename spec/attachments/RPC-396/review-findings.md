# Review: RPC-396 — Scrollable, space-filling help dialog with scrollbar

**Date:** 2026-07-01
**Reviewer:** Claude Code review skill + subordinate reviewer agent
**Status: PASS** (0 critical, 2 warnings — both addressed)

## Cross-feature conflicts resolved (verified)
1. **RPC-027 §B (rounded Cyan border)** — kept the border; RPC-396 test S4's
   `has_scrollbar_glyph` corrected to detect ONLY the `■` thumb (not the border/track `│`,
   which are indistinguishable). `tests/rpc027_dialog_parity_bcd.rs` = 11 passed.
2. **RPC-023 source-shape guard (no `Event::Mouse` in help_dialog.rs)** — mouse-wheel matching
   extracted to `help_dialog_scroll::wheel_direction`; `help_dialog.rs` contains `Event::Mouse`
   only in a comment. `tests/source_shape_rpc023.rs` = 7 passed.

## 🔴 Critical: None

## 🟡 Warnings (addressed)
1. **Arch-note drift** — note claimed `ensure_visible` reuse; impl clamps directly (correct for
   content-scroll with no selection cursor). → **FIXED**: architecture note [1] rewritten to state
   `ensure_visible` is intentionally not used.
2. **S1 coverage range starts at line 96** (a helper close/banner) rather than 102. Cosmetic — the
   range fully encloses the test body and all @step comments. Left as-is (non-blocking).

## 🟢 Observations
- `help_dialog.rs` = 257 LoC, `help_dialog_scroll.rs` = 149 LoC (both < 300).
- No panic on tiny terminals: `fill_rect` + `render_dialog_at` guards verified at 4×4.
- No unwrap/todo/unimplemented/panic in production paths.
- `render_dialog_borderless_at` fully removed (borderless experiment reverted).
- `wheel_direction` is called from `help_dialog.rs:153` (not dead code).
- All 18 @step comments match feature step text word-for-word.
- 2 snapshots regenerated for the new bordered space-filling size; no pending `.snap.new`.

## Coverage Verification
- Feature: 4 scenarios, valid G/W/T, no placeholders, tags `@RPC-396 @tui @dialog @navigation`, doc string present.
- Example Map: 6 rules + 4 examples → 4 scenarios; no unanswered questions; arch notes match impl (after fix).
- Tests: 4/4 scenarios, real assertions (scroll math, buffer content, `■` presence/absence).
- Scenario coverage: 100% (4/4), audit 8/8 files valid.
- Build/Test: full suite 2000 passed / 0 failed; clippy 0 warnings; fmt clean.

## Files Reviewed
- spec/features/scrollable-space-filling-help-dialog-with-scrollbar.feature
- codelet/fspec-tui/tests/help_dialog_scroll_rpc396.rs
- codelet/fspec-tui/src/components/help_dialog.rs
- codelet/fspec-tui/src/components/help_dialog_scroll.rs
- codelet/fspec-tui/src/components/dialog_theme.rs
- codelet/fspec-tui/tests/rpc027_dialog_parity_bcd.rs
- codelet/fspec-tui/tests/source_shape_rpc023.rs
