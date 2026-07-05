# Review: BLOCK-010 — BlocklistView keyboard parity (remove vim j/k; add PageUp/PageDown/Home/End)

**Date:** 2026-07-05
**Reviewer:** Claude Code (fspec review skill) via subordinate reviewer agent
**Status:** ✅ PASS (all warnings fixed)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2 (both fixed)
- 🟢 Observations: 2 (non-blocking, no action)

## 🔴 Critical Issues
None.

## 🟡 Warnings (Should Fix) — FIXED
1. **Stale footer wording in `blocklist-view-framing.feature`** — the architecture
   doc-string (line 10) and Business Rule 4 (line 21) still advertised the removed
   `↑↓/jk: Navigate` footer. The actual `FOOTER_HINT` (render.rs) is now
   `↑↓ Navigate | PgUp/PgDn/Home/End: Scroll | Enter/Space: Toggle Rule | Esc: Close`.
   → ✅ Fixed: both lines updated to the new wording with a BLOCK-010 supersede note.
   Feature re-validated OK. (The framing test only asserts `Enter/Space: Toggle Rule`
   + `Esc: Close`, both still present, so no test change needed.)
2. **`rpc056-blocklist-view-dispatch.feature` still described vim** — the feature
   doc-string (lines 26–28) and Rule 7 (line 56) said "supports j/k navigation".
   → ✅ Fixed: reworded to "arrow-key navigation (plus PageUp/PageDown/Home/End per
   BLOCK-010)". Feature re-validated OK.

## 🟢 Observations (No Action)
1. `blocklist-view-scrolling.feature` Rule 2 listed "Down/Up/j/k" as navigation keys.
   → ✅ Also tidied to "Down/Up, and PageUp/PageDown/Home/End per BLOCK-010" for accuracy.
2. `page_down`/`page_up` compute the step and clamp with `.min(last)` / `saturating_sub`
   rather than looping `move_down()` N times like model_selector. Behaviourally
   equivalent, empty-list safe, DRY satisfied via shared `adjust_scroll()` →
   `scroll_viewport::ensure_visible`. Acceptable divergence — no change.

## Coverage Verification
- Feature file: `spec/features/blocklist-view-keybindings.feature` — OK (G/W/T ordering,
  @BLOCK-010 tag, accurate architecture doc-string, no placeholders). 6 rules → scenarios;
  5 examples → scenarios; 0 open questions.
- Test files: `codelet/fspec-tui/src/views/blocklist/tests.rs` (254–429),
  `codelet/fspec-tui/tests/blocklist_view_rpc056.rs` (reconciled 206–256) — OK.
  Every @step comment matches Gherkin verbatim; assertions verify real behaviour.
- Impl files: `blocklist/mod.rs` (288 lines <300), `blocklist/render.rs` — OK. Vim
  `Char('j'/'k')` arms GONE from production (only the test presses j/k to assert they're
  inert). No unwrap/expect/todo! in production. FOOTER_HINT free of "jk".
- Scenario coverage: 8/8 (100%).

## Verification
- `cargo test -p codelet-fspec-tui` → all suites 0 failed (lib 425 passed; 8 BLOCK-010 +
  2 reconciled rpc056 tests green).
- `cargo clippy -p codelet-fspec-tui --lib --tests` → 0 warnings.

## RPC-056 spec reconciliation (done during implementing)
The two RPC-056 dispatch scenarios that asserted vim j/k were superseded by BLOCK-010:
renamed to "Down advances the focused row" / "Up retreats the focused row, clamped at 0"
and their tests rewired to arrow keys. Coverage re-linked.
