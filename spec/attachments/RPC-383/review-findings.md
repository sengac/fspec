# Epic Review: RPC-383 — TurnContentModal full-screen + scrollable (parity)

**Date:** 2026-06-28
**Reviewer:** Claude Code (fspec review skill) + subordinate reviewer agent
**Work Units Reviewed:** 1 (RPC-383 — no children)
**Build state at review:** all crate tests pass; clippy clean except the pre-existing
`await_holding_lock` warnings in `tests/tui093_default_thinking_restore_parity.rs`; `cargo fmt --check` clean.

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 4
- 🟢 Observations: 3

## RPC-383 Result — WARN

### 🔴 Critical
None. No unwrap()/expect()/panic!/todo!/unimplemented!/TODO/FIXME/HACK in production
code. All files < 300 lines. Feature wired end-to-end and verified.

### 🟡 Warnings
1. **Coverage test line ranges are stale.** The linked `testLines` (e.g. 209-255) point at
   helper code, not the asserting `#[test]` fns (actual ~262-308, etc.). Every test link is
   offset ~50 lines. → FIX: re-link with correct ranges. *(supervisor owns fspec state)*
2. **Rule "PageUp/PageDown scroll by a page and Home/End jump to top/bottom" only partly
   covered.** `End` has a scenario; **PageUp, PageDown, Home have no scenario and no test**
   though implemented (`dispatch_select.rs:42-57`, `dispatch_scroll.rs`). → FIX: add
   scenarios + tests.
3. **Title text/color parity divergence vs TS reference.** TS uses bold-cyan full role
   names (`User Message`/`Assistant Response`/`Tool Output`/`Supervisor Input`); Rust uses
   `You`/`Agent`/`Tool` with per-role accents. Deliberate match to Rust scrollback role
   coloring (established in RPC-382). → RESOLVE: document as an intentional divergence.
4. **Body role-coloring parity gap.** TS colors body lines (user=green, supervisor=magenta,
   tool diff [R]/[A] backgrounds); Rust renders plain `Span::raw`. Out of scope for the two
   named bugs (sizing + scroll). → RESOLVE: create a follow-up work unit.

### 🟢 Observations
1. **Geometry duplication.** The fixed-rect → inner-width → viewport-rows → overflow-narrow
   computation is near-duplicated in `turn_modal.rs::render` and
   `dispatch_scroll.rs::turn_modal_metrics`; they must stay in lockstep or the reducer clamp
   desyncs from what is painted. → FIX: extract a shared geometry helper.
2. Two scrollbar painters exist crate-wide (`scrollback_paint::paint_scrollbar` reused here
   vs `components/list_scrollbar`). No rule violated; possible future consolidation.
3. `dispatch_scroll.rs::wrap_count` re-implements `TurnContentModal::wrap_all` row counting;
   fold into the shared geometry helper (relates to O1).

## Coverage Verification
- Feature file: spec/features/agentview-turn-content-modal-fullscreen-scroll.feature — OK
- Test file: tests/turn_content_modal_fullscreen_scroll_parity_rpc383.rs — OK (ranges stale, W1)
- Impl files: turn_modal.rs, dispatch_select.rs, app/dispatch_scroll.rs, mouse_dispatch.rs — OK
- Scenario coverage: 7/7 linked (but a rule lacks a scenario, W2)

## Disposition (supervisor decisions)
- W1 → fix coverage links (supervisor).
- W2 → add PageUp/PageDown/Home scenarios + tests (ACDD: specifying → testing → impl verify → validating).
- W3 → document intentional title divergence as a RPC-383 architecture note.
- W4 → create follow-up backlog work unit for body role-coloring parity.
- O1 + O3 → extract a shared geometry/row-count helper used by both render and reducer.
- O2 → noted; no action this card.

## Fix Results (post-review)
- 🟡 W1 Stale coverage line ranges → ✅ FIXED: unlinked all 7 stale mappings; re-linked test+impl for all 10 scenarios with correct current ranges. `audit-coverage` → 20/20 files found, all mappings valid; `show-coverage` → 100% (10/10).
- 🟡 W2 PageUp/PageDown/Home untested → ✅ FIXED: added 3 scenarios to the feature file and 3 tests (turn_content_modal_fullscreen_scroll_parity_rpc383.rs:433-458, 459-491, 492-528) with exact-text @step comments. All pass.
- 🟡 W3 Title divergence → ✅ RESOLVED: recorded as an intentional architecture-note decision on RPC-383 (Rust reuses RPC-382 scrollback role coloring).
- 🟡 W4 Body coloring parity gap → ✅ DEFERRED: created follow-up bug RPC-384.
- 🟢 O1+O3 Geometry/row-count duplication → ✅ FIXED: extracted `turn_modal_geometry()` + `wrap_row_count()` in dialog_theme_rows.rs; both `TurnContentModal::render` and `App::turn_modal_metrics` now call the single helper. Local `wrap_count` deleted.
- 🟢 O2 Two scrollbar painters → noted, no action.

## Final Verification
- Full crate tests: 202 suites pass, 0 failed (incl. 10/10 RPC-383 scenarios) ✅
- `cargo fmt --check`: clean ✅
- `cargo clippy --all-targets`: only the 5 pre-existing `await_holding_lock` warnings in tests/tui093_default_thinking_restore_parity.rs (lines 63,107,136,173,223) — no new warnings ✅
- `fspec validate`: all 1507 feature files valid ✅
- Coverage: 100% (10/10), audit-coverage all mappings valid ✅
- All touched files < 300 lines ✅
