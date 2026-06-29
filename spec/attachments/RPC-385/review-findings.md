# Review: RPC-385 — Spawned subordinate agents are not registered/visible in the Rust TUI

**Date:** 2026-06-29
**Reviewer:** Claude Code (fspec review skill) — dedicated reviewer agent
**Work Units Reviewed:** 1 (standalone bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 6
- **Verdict: PASS**

## 🔴 Critical Issues
None.

## 🟡 Warnings (Should Fix)
1. **Duplicated idempotency logic across two layers.** Both `handle_session_created`
   (dispatch_create_session_dialog.rs) and `AgentViewStore::append_session`
   (agent_view.rs) independently guard against duplicate tabs (different idioms:
   `session_context_for` vs `open_sessions.iter().any`). Defensible defense-in-depth
   (the dispatch guard ALSO short-circuits chrome/supervisor/pending-input fetches,
   which the store cannot), but the invariant is enforced in two places with no
   cross-reference. → FIX: add brief comments cross-referencing the two, documenting
   that `append_session`'s guard is the authoritative store-level invariant and the
   dispatch guard is the side-effect-suppression optimization.

2. **`SUPERVISOR_BROADCAST_CAPACITY` reused for the session-created channel**
   (session_manager.rs `new()`), while degenerate/closed receivers elsewhere use a
   capacity-1 channel. Functionally fine, but the capacity choice for the REAL
   channel is implicit and matters (the lag-recovery test floods 200 events). →
   FIX: add a one-line comment explaining why the supervisor capacity is the right
   bound for session-creation events.

## 🟢 Observations
1. Scenario #2 test genuinely exercises the FULL path (App boot → real fifth
   subscriber → push_session_created → wait_for_action for subscriber-produced
   Action::SessionCreated → dispatch → append), not a bare store append. Documented
   in the test header. Excellent.
2. Broadcast fired on BOTH creation paths: `create_session_with_id` and
   `create_isolated_session_with_id`. Cross-cutting requirement satisfied.
3. Ordering safe: `created_info = session.get_info()` captured before insert,
   broadcast fires after insert; id-only idempotent append means no race.
4. Graceful degradation confirmed on every default/closed-receiver path
   (core handle default, FspecBackend trait default, websocket empty_broadcast_rx,
   embedded fallback, SharedFspecService): subscriber observes RecvError::Closed →
   break, no panic.
5. `cargo clippy -p codelet-fspec-tui -p codelet-sessions` → 0 warnings. No
   todo!()/unimplemented!(). Broadcast send errors (no subscribers) ignored with
   documented rationale.
6. `app_bootstrap_rpc009.rs` subscriber-count assertions correctly updated 4→5 with
   explanatory comments.

## Coverage Verification
- Feature files: `sessionmanager-session-created-broadcast.feature` (scenario #1) — OK;
  `agentview-spawned-subordinate-session-registration.feature` (scenarios #2-5) — OK.
  (Placeholder template tag line `@critical @component @feature-group` on the broadcast
  feature — FIXED post-review by the supervisor.)
- Test files: sessions + fspec-tui parity tests — OK; @step comments match.
- Impl files: all wired end-to-end, all under 300 lines (only small additions to
  pre-existing large files) — OK.
- Scenario coverage: 5/5 (4/4 agentview + 1/1 sessionmanager), 100%.

## Fix Results
- Placeholder tag line on broadcast feature → ✅ Fixed (removed by supervisor).
- 🟡 W1 (duplicated idempotency cross-reference comments) → ✅ Fixed (see below).
- 🟡 W2 (session-created channel capacity comment) → ✅ Fixed (see below).

## Final Verification
- Targeted tests pass: ✅ (sessions 1/1, fspec-tui 5/5)
- Wider suites: ✅ no regression (fspec-tui all green; sessions only pre-existing
  `rpc073_list_providers_wiring` Tokio-runtime failure, confirmed present on base)
- clippy/fmt clean on touched crates: ✅
- Coverage complete: ✅ 100%
- Feature files valid: ✅
