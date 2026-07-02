# Review Findings: RPC-408 — send_hitl_response discards the user's answer

**Date:** 2026-07-02
**Reviewer:** Claude Code (fspec review skill, parallel reviewer 74b5aff0)
**Status:** PASS (0 critical, 4 warnings, 7 observations)

## 🔴 Critical Issues
None. Mapping verified correct (option label → `selected:[label]/other:None`; free text → `selected:[]/other:Some(text)`; keyed by pending question id; `tracing::warn!` on id mismatch; fallback without pending request; NEVER constructs Cancelled). 5/5 tests pass. Facade round-trip restored (wrapper.rs:245-252 now receives Answered → JSON success). No send/clear race (request read before channel delivery; agent loop clears after unblock). Esc still sends nothing (hitl_dialog.rs:181-187).

## 🟡 Warnings (Should Fix)
1. **Coverage line ranges slightly inaccurate** — Scenario 1 links `rpc408_hitl_response_answer_mapping.rs:91-128` but the test spans 89-126; Scenario 5 links `269-299` but the file is 297 lines (range past EOF; actual test 267-297). Re-link all five scenarios against current line numbers.
2. **Rule [5] (napi parity) has no scenario or test** — parity is enforced only by a comment. Reviewer verified manually today (napi at session_bindings.rs:1720-1750 only sends Cancelled when cancelled:true). Fix minimally: add a scenario + test that locks the wire-path mapping invariants (never-Cancelled already locked; add the label-vs-free-text discrimination table as an explicit parity contract), OR extract a shared `label→HitlAnswer` helper. Full cross-path parity test is impractical (napi input shape differs: structured HitlResponseInfo vs flat id/value).
3. **Scenario 5 has no `When` step** (feature lines ~68-71: Given → Then). Add e.g. "When the source of send_hitl_response is inspected".
4. **`handle_impl.rs` is 1,897 lines** — far over the 300-line guideline. Pre-existing (RPC-408 added ~50 lines). NOTE (supervisor): decomposition is out of scope for this bug-fix review; recorded as a follow-up refactor candidate rather than fixed here.

## 🟢 Observations (Nice to Have)
1. Free-text mapping ignores `allow_text_input` (any non-label → other). Benign; napi has same latitude; parity holds.
2. No true mapping duplication with napi (different input shapes); shared helper only needed if parity enforcement is desired.
3. Pre-existing expect()/recv-error→Cancelled fallback in background_session.rs:1049-1058 — adjacent, correct semantics (channel disconnect = cancel).
4. No race between send and pause-state clearing — verified sound.
5. Esc-sends-nothing assumption verified accurate.
6. Facade round-trip confirmed end-to-end.
7. Multi-question wire support correctly out of scope (documented in three places).

## Coverage Verification
- Feature file: spec/features/hitl-response-answer-mapping.feature — OK (@RPC-408, arch doc string; W3 noted)
- Test file: codelet/sessions/tests/rpc408_hitl_response_answer_mapping.rs — OK (5 tests 1:1, exact @step text, real blocked-thread behavioral assertions; 5 passed/0 failed in 4.50s)
- Impl: codelet/sessions/src/handle_impl.rs:748-816 — OK (verified to be the actual mapping logic)
- Scenario coverage: 5/5 — line-range drift per W1

## Fix Results (2026-07-02, remediation worker 2fc6fd5f)

- **W1 (stale coverage ranges) — FIXED.** All scenarios re-linked to verified current line numbers in `codelet/sessions/tests/rpc408_hitl_response_answer_mapping.rs` (scenario 1 → 89-126, scenario 5 "never delivers Cancelled" → 339-371 after the new test landed above it; every range read and verified); impl range `handle_impl.rs:748-816` re-confirmed accurate.
- **W2 (napi parity unlocked) — FIXED (red-green).** New scenario "Option label versus free text discrimination is the wire-path parity contract" + one table-driven test (test lines 268-333) asserting, for a pending question with options ["Yes","No"] and allow_text_input: value=="Yes" → selected:["Yes"]/other:None; value=="No" → selected:["No"]/other:None; anything else → selected:[]/other:Some(...). Comment cites `codelet/napi/src/session_bindings.rs:1720-1750` as the parity reference. No cross-crate harness, napi untouched (per scope).
- **W3 (scenario 5 missing When) — FIXED.** Added "When the source of send_hitl_response is inspected" to the feature scenario and matching `@step` comment in the test.
- **W4 (handle_impl.rs 1,897 lines) — OUT OF SCOPE per supervisor.** Recorded as an architecture note on RPC-408 as a follow-up refactor candidate.
- **Verification:** `cargo test -p codelet-sessions` green (new test failed red first, then passed); clippy + fmt clean; coverage 100% (6/6 scenarios incl. the new one). Card cycled done → implementing → validating → done.
