# Review Findings: RPC-405 — MultiLineInput lacks soft-wrap

**Date:** 2026-07-02
**Reviewer:** ACDD compliance reviewer (review-skill, parallel worker)
**Status:** WARN → fixes applied (see Fix Results)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 4
- 🟢 Observations: 5

## 🟡 Warnings
1. **Stale coverage impl/test line ranges (post-RPC-404 drift)** — RPC-404's `hardware_cursor_in` insertion shifted `multiline_input_render.rs` / `agent.rs` line numbers after coverage was linked:
   - "Moving the cursor to the top scrolls the six-row window up" → linked 47-53, actual `sync_viewport` now 71-77
   - "Empty buffer shows the one-row placeholder" → linked 83-117, actual placeholder branch inside `render_with_prompt` (~107-140)
   - "Input height derives from the exact renderer body width" → linked agent.rs 225-226, actual 222-223
   - Test ranges drifted slightly (e.g. S1 body 63-106 vs linked 59-101)
2. **Scratch repro file left in tree** — `codelet/fspec-tui/tests/zz_repro_multiline_render.rs`: test 1 has NO assertions (eprintln only); test 2 subsumed by the RPC-405 suite. Delete.
3. **`MultiLineInput::visible_rows()` doc stale / production-dead** — logical-line variant no longer feeds the layout; only tests call it. Update doc to state it is the logical-line count, superseded by `visible_rows_for_width` for layout.
4. **`body_width = width - 4` geometry duplicated in 3 places** — agent.rs:222, `hardware_cursor_in`, and the pad+prompt carve path. Extract a single shared helper.

## 🟢 Observations
1. Segmentation runs ~4-5× per frame — acceptable for an input box; cache only if buffers grow.
2. History-recall scenario simulated via `set_value` (same call the dispatcher makes) — acceptable, documented in-test.
3. Supplementary test clearly marked; no @step comments by design.
4. Architecture note line-number drift (agent.rs:228 → 223) — cosmetic.
5. `text_wrap.rs` (scrollback) wraps by char-count proxy vs `multiline_wrap.rs` unicode-width — divergence documented upstream.

## Fix Results
- 🟡 1 Coverage drift → ✅ Fixed: re-linked all affected scenarios with post-fix line numbers (audit-coverage clean).
- 🟡 2 Scratch file → ✅ Fixed: `zz_repro_multiline_render.rs` deleted.
- 🟡 3 Stale doc → ✅ Fixed: `visible_rows()` doc rewritten (logical-line count; layout uses `visible_rows_for_width`).
- 🟡 4 Geometry duplication → ✅ Fixed: shared `input_body_width(area)` helper (with `INPUT_PAD_X`/`PROMPT_WIDTH` consts) in `multiline_input_render.rs`; all sites (`agent.rs` layout, `hardware_cursor_in`, `render_with_prompt`) consume it; parity unit test added.

## Final Verification
- Full crate tests pass ✅ (0 failures)
- clippy --all-targets clean ✅, fmt clean ✅
- audit-coverage: all mappings valid ✅
- Feature files valid ✅
