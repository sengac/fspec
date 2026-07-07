# Review: RPC-415 — Live streaming dies permanently after first auto-reconnect

**Date:** 2026-07-07
**Reviewer:** ACDD compliance reviewer (fspec review skill)
**Status:** WARN (0 critical, 2 warnings, 3 observations)

## 🔴 Critical Issues
None. Core fix correct and wired end-to-end: `Action::Reconnected` → `dispatch.rs` →
`handle_reconnected()` (`dispatch_reconnect.rs:125-152`) aborts+drains dead handles then
re-invokes the single `spawn_subscriber_tasks()` path (DRY, shared with bootstrap).
Flapping idempotency real (drain-before-respawn keeps count at 5). 5+4 targeted tests green.
No unwrap/expect/todo!/unimplemented! in prod; all files < 300 lines.

## 🟡 Warnings (Must Fix)
1. **Coverage impl line ranges are wrong for every scenario.** `.feature.coverage` maps
   all four scenarios to `dispatch_reconnect.rs:28–52`, which is `handle_disconnected`
   (RPC-416 code), NOT the respawn. Actual respawn is `dispatch_reconnect.rs:125–152`
   (`handle_reconnected`). Re-link to 125–152 (plus the shared `bootstrap.rs` spawn path).
2. **Stale "three" docstrings remain in `bootstrap.rs`.** Header (`bootstrap.rs:1–8`) and
   the `spawn_subscriber_tasks` doc (~line 134) still say "three subscriber tasks
   (work_units_rx / chunks_rx / logs_rx)" — it actually spawns FIVE. Same stale-count
   defect class RPC-415 explicitly targeted; `components/mod.rs:162` was fixed but these
   two were missed. Correct "three" → "five".

## 🟢 Observations
1. Defensive `abort()` on already-dead handles — good belt-and-suspenders.
2. Scenario 3 uses two `Then` keywords where the 2nd is logically `And` (style nit; @step
   comments mirror verbatim so coverage matching holds).
3. Strengthened `auto_reconnect_slice2_rpc011.rs` step now has a genuine assertion
   (count==5 AND post-reconnect broadcast reaches bus).

## Fix Plan
- Re-link coverage impl ranges to `dispatch_reconnect.rs:125-152`.
- Fix the two stale "three" docstrings in `bootstrap.rs`.
- (Optional) convert scenario-3 2nd `Then` → `And` + matching @step.
