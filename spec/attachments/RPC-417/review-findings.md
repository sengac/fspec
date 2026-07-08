# Epic Review: RPC-417 — COMPACTED header badge auto-hide

**Date:** 2026-07-08
**Reviewer:** Claude Code (fspec review skill) + subordinate reviewer 4ccab45b
**Work Units Reviewed:** 1 (RPC-417, standalone bug — no children)

## Summary
- 🔴 Critical: 1 issue (rpc094 source-shape budget breach)
- 🟡 Warnings: 3 issues (coverage line range, thin impl links, seq narrative mismatch)
- 🟢 Observations: 2 (acceptable duplication, handler self-clear correctness)

## Work Unit Results

### RPC-417: COMPACTED header badge auto-hide — WARN

Implementation is functionally correct, faithfully mirrors `dispatch_reconnect.rs`, and all
RPC-417 (5/5) + RPC-100 (5/5) tests pass. One source-shape budget test fails.

#### 🔴 Critical
1. **`scrollback_scroll_rpc094::rpc094_source_shape_every_touched_module_under_300_lines` FAILS.**
   `codelet/fspec-tui/src/components/mod.rs` is now **1221 lines** vs the test's hardcoded
   `<= 1206` budget (`tests/scrollback_scroll_rpc094.rs:636`). RPC-417 added the
   `ClearCompactionReduction` variant (+2). HEAD was already 1219 (+13 pre-existing overflow
   from a prior card that forgot to bump). Per the test's own per-card tally convention
   (`scrollback_scroll_rpc094.rs:588-633`), RPC-417 must append its `+2` tally line and raise
   the assertion so the touched-module gate passes. **Fix owned by RPC-417 because it edits
   the file.**

#### 🟡 Warnings
1. **Coverage link line range off for scenario 1.** `show-coverage` reports test range
   `137-190` for "The COMPACTED badge auto-hides…" but the test fn spans **135-186**
   (line 190 is inside the next test). Re-link with `135-186`.
2. **Scenario→impl links thin/mismatched.** The "auto-hides after 10s" scenario links only
   `dispatch_compaction_hide.rs:36-70` but the arming behaviour it exercises lives in
   `dispatch_stream_chunks.rs:141-159`; the "paused tokio time" scenario (which truly proves
   arming) doesn't link `arm_compaction_hide`. Routing (`dispatch.rs`) and the `state.rs`
   field are unlinked. Improve representativeness.
3. **Seq narrative (0/1) doesn't match implementation (1/2).** `bump_compaction_reduction_seq`
   does `or_insert(0)` then `wrapping_add(1)`, so the FIRST compaction yields seq `1`, not `0`.
   The Gherkin (`agentview-compaction-badge-auto-hide.feature:54-55`) and test comments say
   "seq 0 / seq 1". The test only survives because it hardcodes `seq: 0` (always < current →
   always stale). Make the stale-seq test read the real seq via
   `compaction_reduction_seq_for` and dispatch `current - 1`, and correct the Gherkin/comment
   wording so the narrative matches reality.

#### 🟢 Observations
1. `arm_compaction_hide` ~ `arm_reconnect_dismiss` duplication is acceptable (per-session
   HashMap vs single Option); dossier acknowledges it. No action.
2. Handler self-clear seq interaction verified correct: guard confirms current seq before
   `clear_compaction_reduction` re-bumps; only invalidates future stale fires.

## Coverage Verification
- Feature file: `spec/features/agentview-compaction-badge-auto-hide.feature` — OK (valid, tagged)
- Test file: `codelet/fspec-tui/tests/agentview_compaction_badge_auto_hide_rpc417.rs` — ISSUE (Warning 1 line range)
- Impl files: `dispatch_compaction_hide.rs`, `dispatch_stream_chunks.rs`, `chrome_state.rs` — ISSUE (Warning 2 representativeness)
- Scenario coverage: 5/5

## Build & Test Results (supervisor-verified)
- cargo build -p codelet-fspec-tui: ✅ clean
- rpc417 tests: 5 passed / 0 failed
- rpc100 regression: 5 passed / 0 failed
- rpc094 source-shape: 11 passed / 1 FAILED (budget breach — Critical 1)

---

## Fix Results (post-review, supervisor-verified)

### RPC-417
- 🔴 Critical 1 (rpc094 budget) → ✅ Fixed: appended RPC-417 `+2` tally line + reconciliation
  note (measured 1219 exceeded itemized 1206 by 13 due to prior under-count), raised assertion
  `1206 → 1221`, updated panic message. `scrollback_scroll_rpc094`: **12/12 pass**.
- 🟡 Warning 1 (coverage test line ranges) → ✅ Fixed: recomputed & re-linked all 5 scenarios to
  true test-fn spans (135-186, 190-235, 239-291, 295-351, 355-401).
- 🟡 Warning 2 (thin impl links) → ✅ Fixed: each scenario now links its representative impl
  across 4 files (dispatch_stream_chunks arm, dispatch_compaction_hide arm/guard, dispatch
  routing, chrome_state clear).
- 🟡 Warning 3 (seq narrative 0/1 vs impl 1/2) → ✅ Fixed: reworded feature scenario + example
  to remove magic "seq 0/1"; test now captures the real seq via `compaction_reduction_seq_for`
  and dispatches the genuinely-superseded seq; `// @step` comments realigned; feature validates.

## Final Verification
- cargo build: ✅ clean
- agentview_compaction_badge_auto_hide_rpc417: ✅ 5/5
- agentview_session_header_compaction_percentage_rpc100 (regression): ✅ 5/5
- scrollback_scroll_rpc094 (source-shape): ✅ 12/12
- Coverage: ✅ 100% (5/5), corrected ranges + 4 impl files
- Feature file valid: ✅
- Work unit status: **done**
