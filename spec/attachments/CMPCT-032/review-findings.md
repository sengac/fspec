# CMPCT-032 Review Findings

**Reviewer:** Claude Code (fspec review skill) via subordinate agent 85f23ad8-3274-4adc-aa6a-05a2afab38ff
**Date:** 2026-04-17
**Work Unit:** CMPCT-032 — Compaction triggering broken after CMPCT-023..031 refactor — FinalResponse path bypasses recovery

## Status: PASS (with minor warnings)

## 🔴 Critical Issues (Must Fix)

None.

## 🟡 Warnings (Should Fix)

1. **Grep-only assertions for control flow** — `codelet/cli/tests/compaction_trigger_reliability_test.rs:235-256, 332-342, 425-429, 486-489, 554-630, 690-697, 907-912`. Most structural assertions match against `stream_loop.rs` source text (e.g. "the window between `// Normal case:` and `emit_done_with_stop_reason(` contains `compaction_needed`" / "`in_loop_compaction_restart!(policy)` appears ≥ 3 times"). These prove the source contains the right tokens, not that the FinalResponse branch or the post-loop safety net ACTUALLY route through `begin_compaction_recovery` at runtime. A developer who renames a macro, moves the comment, or replaces the call with dead code could still pass these assertions. A real runtime test would require a mock `Agent` + fake `Stream` yielding `Ok(FinalResponse)` / `None` / errors on demand — the test file's doc-comment acknowledges this limitation (lines 20-37), but the coverage gap remains.

2. **Duplicated flag-reset logic** — `codelet/cli/src/interactive/stream_loop.rs:634-640` (macro) and `codelet/cli/src/interactive/stream_loop.rs:1900-1906` (post-loop safety net). Both blocks do the same `state.compaction_needed = false; state.input_tokens = session.token_tracker.input_tokens; state.cache_read_input_tokens = 0; state.cache_creation_input_tokens = 0; state.output_tokens = 0;`. Could be extracted into a private helper (e.g. `reset_token_state_after_compaction(&token_state, &session)`). Low-severity DRY issue.

3. **File size growth** — `codelet/cli/src/interactive/stream_loop.rs` went from 1798 → 1947 lines (+149). The project guideline (TypeScript) is 300 lines; Rust has no hard rule but this file has been a monolith for a while. The CMPCT-032 fix makes it worse. Consider extracting the post-loop safety-net block into a helper in `recovery_compaction.rs` (e.g. `post_loop_compaction_safety_net(...)`) — same helper could be invoked from both the FinalResponse branch and the post-loop position once factored properly.

4. **Partial-content risk note (resolved, but worth documenting)** — `codelet/cli/src/interactive/stream_loop.rs:1324 assistant_text.clear();`. The inline comment explains the clear is safe because `handle_final_response` at line 1131 already appended the text to `session.messages`. The assertion is correct, but a bare `assistant_text.clear()` could be alarming at a glance — consider adding a `debug_assert!` that `session.messages.last()` is the Assistant message just appended, or extracting into a named helper `mark_assistant_text_already_flushed(&mut assistant_text)` with a `#[inline]` comment explaining the precondition.

5. **No end-to-end runtime proof for FinalResponse branch** — `codelet/cli/tests/compaction_trigger_reliability_test.rs:191-296`. The test `final_response_branch_triggers_recovery_when_compaction_needed_is_set` proves (a) the source contains the right tokens via grep, and (b) `begin_compaction_recovery` + `compaction_retry_prompt` behave correctly in isolation. It does NOT prove that when a real `stream.next()` yields `Ok(FinalResponse)` with `compaction_needed=true`, control flow actually reaches the new block. A fake `Stream` implementation (yielding a scripted sequence of `StreamingChoice`-like items) would close this gap. Non-blocking because the structural assertions + helper behavioural tests together are highly suggestive of correctness, but the gap exists.

## 🟢 Observations (Nice to Have)

1. **Post-loop failure handling is conservative and correct** — `codelet/cli/src/interactive/stream_loop.rs:1893-1914`. When `execute_compaction_and_capture_events` fails, the flag is intentionally NOT cleared. This means the next turn's pre-prompt compaction check will still see `compaction_needed=true` and can retry. Silently clearing on failure would be a worse regression than the original bug. The `warn!` message clearly explains to the operator what happened.

2. **`warn!` messages are rich and diagnostic** — `codelet/cli/src/interactive/stream_loop.rs:1319-1322, 1859-1866, 1894-1896, 1910-1913`. Each safety-net firing produces an operator-facing log identifying exactly which branch missed the check and what happened next.

3. **All three CMPCT-023..028 error-arm paths preserved** — `codelet/cli/src/interactive/stream_loop.rs:1124` (Path D), `1463` (Path C), `1529` (Path B). Test `in_loop_macro_handles_*` scenarios assert these via source grep at lines 335, 427, 487 — they verify the CMPCT-027 comments survive. Plus `deleted_compaction_retry_file_must_remain_deleted` guards against reverting CMPCT-027.

4. **Interrupt priority correctly enforced on both new paths** — FinalResponse branch guard (`stream_loop.rs:1313` `!is_interrupted.load(Acquire) && ...`) and post-loop guard (`stream_loop.rs:1853` same pattern). Matches Rule[3] and Example[5]. The `user_interrupt_takes_priority_over_compaction_recovery` test asserts the guard exists in the post-loop window via structural grep and the behavioural preconditions (flag=true, is_interrupted=true) hold.

5. **Per-model threshold wiring intact** — `stream_loop.rs:286` still calls `resolve_compaction_threshold`. Tests validate CTX-007 behaviourally at the override boundary, the default boundary, and prove the override value actually differs from the default formula (assert_ne! at line 807-810).

6. **Hook reset on successful post-loop compaction** — `stream_loop.rs:1899-1906` clears the flag, resets input/cache/output counters, so the next `run_agent_stream` invocation starts with a fresh `compaction_needed=false`. Matches Rule[5] and Example[8]. The `after_recovery_restart_stream_does_not_immediately_re_trigger_the_hook` test validates this contract behaviourally.

7. **Comment quality is excellent** — The inline comments at `stream_loop.rs:1293-1312` and `1828-1852` explain WHY the block exists, which branches it catches, the pre-CMPCT-032 state, the interrupt-priority reasoning, and the `assistant_text`/token-tracker side effects.

8. **Four call sites for `in_loop_compaction_restart!(policy)`** — one more than the test requires (≥3). Beyond Paths B/C/D (lines 1124, 1463, 1529), the new FinalResponse branch invocation at line 1337 brings the count to 4, proving the fix unifies recovery through the same retry-budget / fresh-hook / fresh-stream pathway rather than re-implementing recovery inline.

## Coverage Verification

- **Feature file**: `spec/features/compaction-trigger-reliability.feature` — OK (9 scenarios, `@CMPCT-032` tag on line 5, architecture doc string on lines 8-13, no prefill placeholders, all Given/When/Then correctly ordered, Background user-story card on lines 41-44)
- **Test file**: `codelet/cli/tests/compaction_trigger_reliability_test.rs` — OK (10 tests incl. deletion regression guard at line 949, 59 `@step` comments mapping to Gherkin steps verbatim, feature header on line 2, all tests pass). Minor concern: structural grep assertions are the primary proof mechanism for control-flow — see Warnings #1 and #5.
- **Impl file**: `codelet/cli/src/interactive/stream_loop.rs` — OK (FinalResponse guard at 1293-1339 + post-loop safety net at 1828-1916, both production-mode, both with rich `warn!` observability; no `unwrap()` / `todo!()` / `unimplemented!()` in new code; no `#[cfg(debug_assertions)]` gating around production-critical behaviour — only one occurrence of that string remains at line 1840 inside a comment explaining the old behaviour). Minor concerns: DRY & file size — see Warnings #2, #3.
- **Scenario coverage**: 9/9

## Files Reviewed

- `spec/attachments/CMPCT-032/research-findings.md` (full file, 131 lines)
- `spec/features/compaction-trigger-reliability.feature` (full file, 127 lines)
- `codelet/cli/tests/compaction_trigger_reliability_test.rs` (full file, 962 lines)
- `codelet/cli/src/interactive/stream_loop.rs` (targeted reads around lines 595-695, 1055-1155, 1130-1345, 1790-1920; plus grep surveys for `unwrap()`, `todo!()`, `unimplemented!()`, `cfg(debug_assertions)`, `handle_final_response`, `in_loop_compaction_restart`, `process_turn_annotations`, `resolve_compaction_threshold`)
- `codelet/cli/src/interactive/recovery_compaction.rs` (targeted reads around lines 155-355, 400-460)
- `codelet/cli/src/interactive/stream_handlers.rs` (lines 240-260 for `handle_final_response`)
- `git diff HEAD -- codelet/cli/src/interactive/stream_loop.rs` (full 166-line diff)
- `fspec show-work-unit CMPCT-032`
- `fspec show-coverage compaction-trigger-reliability` (100%, 9/9)

## Build & Test Verification

- **cargo build**: PASS (clean; zero warnings introduced by CMPCT-032 changes)
- **cargo test --package codelet-cli**: PASS — 41 test binaries all green, zero failures. Unit: 152/152. Full integration run: all `test result: ok. N passed; 0 failed`
- **cargo test --test compaction_trigger_reliability_test**: PASS — 10/10

## Overall Recommendation

Ship it. No critical issues. The 5 warnings describe incremental improvements (DRY extraction, runtime stream-level integration test, minor hygiene). None block the production fix from being shipped — the regression introduced by commit dc3d5934 is closed, the tests pin the fix against future regressions (both structurally and behaviourally), and the safety nets are now production-mode (not `#[cfg(debug_assertions)]`).

The behavioural tests against the real `begin_compaction_recovery`, `execute_compaction`, `classify_compaction_branch`, `resolve_compaction_threshold`, and `CompactionHook` helpers provide strong end-to-end coverage of every helper the new safety nets invoke. The remaining runtime gap (stream-level fake Stream test) is acknowledged in the test file's doc-comment and is non-trivial to close (~1000 lines of mock rig infrastructure).
