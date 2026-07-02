# Review Findings: RPC-404 — Hardware cursor containment

**Date:** 2026-07-02
**Reviewer:** ACDD compliance reviewer (review-skill, parallel worker)
**Status:** PASS (warnings fixed — see Fix Results)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 3
- 🟢 Observations: 3

Verified correct end-to-end: geometry parity between layout, paint and cursor mapping; clamp guarantees containment (saturating arithmetic, degenerate rect cases safe); call-site order in app/events.rs draw closures correct (render → sync_viewport → cursor_position → set_cursor_position); no stale logical-row cursor math remains; no unwrap/expect/panic in touched production code.

## 🟡 Warnings
1. **`- 4` body-width geometry duplicated** between agent.rs and `hardware_cursor_in` (same as RPC-405 warning 4) — extract shared helper/constants.
2. **Coverage test line ranges overshoot** by ~5 lines (separator comments of the next scenario included); scenario 5 exact.
3. **Scenario 5's first `@step Given` annotates a comment-only line** — the harness construction happens inside the loop; move the annotation onto executable code.

## 🟢 Observations
1. Design-doc sketch inlined the fix in `cursor_position()`; implementation delegates to `MultiLineInput::hardware_cursor_in()` — matches architecture-note intent, keeps agent.rs under the LoC cap. No action.
2. `agent.rs` at 296 lines — next change requires refactoring first.
3. Five tests map 1:1 to scenarios with real coordinate assertions (exact x/y, 4×3 containment grid).

## Fix Results
- 🟡 1 Geometry duplication → ✅ Fixed: shared `input_body_width(area)` helper consumed by `hardware_cursor_in` (see RPC-405 findings).
- 🟡 2 Coverage ranges → ✅ Fixed: re-linked with exact test body ranges.
- 🟡 3 @step placement → ✅ Fixed: Given step annotation moved onto the executable harness construction.

## Final Verification
- rpc404 suite 5/5, rpc405 suite green, full crate 0 failures ✅
- clippy/fmt clean ✅, audit-coverage valid ✅
