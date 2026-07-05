# Review: BLOCK-011 — BlocklistView mouse-wheel scroll support

**Date:** 2026-07-05
**Reviewer:** Claude Code (fspec review skill) via subordinate reviewer agent
**Status:** ✅ PASS (no issues)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 0
- 🟢 Observations: 3 (non-blocking, no action)

## 🔴 Critical Issues
None.

## 🟡 Warnings
None.

## 🟢 Observations (No Action)
1. **Rule 3 (1×–5× acceleration ramp) has no dedicated scenario/test.** Exercised
   indirectly (single-event tests assert the deterministic 1× step). Reasonable —
   time-based acceleration is hard to assert deterministically and the ramp itself is
   covered by `WheelVelocity::step_at` tests in `scroll_viewport.rs` (RPC-028). The
   shared primitive is reused, not reinvented.
2. `mod.rs` is exactly 288 lines — under the 300 ceiling. The `handle_mouse`
   extraction into `mouse.rs` (39 lines) was the correct DRY/size move.
3. Coverage lists impl `mouse.rs:17–40` while the file is 39 lines — cosmetic; the
   range covers the full `handle_mouse` body. No functional impact.

## Coverage Verification
- Feature file: `spec/features/blocklist-view-mouse-scroll.feature` — OK (G/W/T ordering,
  @BLOCK-011 tag, accurate architecture doc-string, no placeholders). 5 rules, 5 examples,
  5 scenarios; 0 open questions.
- Test file: `codelet/fspec-tui/src/views/blocklist/tests.rs` (529–641) — OK. Every @step
  comment matches Gherkin verbatim; assertions verify real behaviour (`selected_index`,
  `scroll_offset()` window containment, `BlocklistEvent::Ignored`,
  `EventResult::is_consumed()`); the navigator test drives the real
  `Navigator::handle_blocklist_event`.
- Impl files: `blocklist/mouse.rs`, `blocklist/mod.rs` (wheel field + mod decl),
  `navigator_events.rs` (149–156), `scroll_viewport.rs` (WheelVelocity derive) — OK.
  No unwrap/expect/todo!; no dead code/unused imports; wheel maps
  ScrollUp/ScrollDown→move_up/move_down via WheelVelocity, non-wheel→Ignored; navigator
  routes Event::Mouse BEFORE the Event::Key guard and translates Consumed→consumed().
- Scenario coverage: 5/5 (100%).

## Cross-Cutting (wired end-to-end)
`handle_mouse` is reachable at runtime: `navigator.rs:106`
`ViewMode::Blocklist => self.handle_blocklist_event(event)` → mouse branch →
`self.blocklist.handle_mouse`. The `WheelVelocity` derive change is additive/safe; no
source-shape/line-count test is pinned to `scroll_viewport.rs`.

## Verification
- `cargo test -p codelet-fspec-tui` → 2204 passed, 0 failed (5 BLOCK-011 tests green).
- `cargo clippy -p codelet-fspec-tui --lib --tests` → 0 warnings.
