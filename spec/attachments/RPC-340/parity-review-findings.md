# RPC-340 — Model selector scroll fix: TS-parity review findings

**Date:** 2026-06-20
**Reviewer:** Parallel ACDD review worker (impl-vs-TS comparison)
**Status:** PASS (minor warnings). Core parity met; viewport math bit-exact with TS; no dead scroll_offset.

## 🔴 Critical
None. No prod `unwrap()/panic!/todo!()`. `ensure_visible` (`scroll_viewport.rs:56-59`) matches TS
`useModelSelectorState.ts:228-229,243-244`.

## 🟡 Warnings (Should Fix)
1. **Dead helper `ModelSelectorView::visible_rows_for` (`mod.rs:673-676`)** — zero callers, computes the
   exact formula the arch note says NOT to use for scroll math. Misleading dead code. **Fix:** delete it.
2. Mouse-wheel is net-new (TS keyboard-only) — acceptable, unaccelerated (no WheelVelocity ramp). Observation.
3. **Header-reveal branch (`mod.rs:164-168`)** has no dedicated scenario. **Fix:** add an explicit assertion that
   the leading provider header is visible after scrolling to the top.

## 🟢 Observations
- Explicit clamp to `total - visible` (`scroll_viewport.rs:62-65`) is a robustness improvement over TS.
- Filter reset uses `ensure_visible` (intended deviation per answered question, feature lines 38-43).
