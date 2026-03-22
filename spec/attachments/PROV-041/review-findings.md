# Epic Review: PROV-041 — Thinking token exhaustion recovery

**Date:** 2026-03-22
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 4 issues across 1 work unit
- 🟡 Warnings: 3 issues across 1 work unit
- 🟢 Observations: 2

## Work Unit Results

### PROV-041: Thinking token exhaustion recovery — Status: FAIL

## 🔴 Critical Issues (Must Fix)

1. **Retry `continue` on exhausted stream is a no-op — no retry actually happens** (stream_loop.rs:1635)
   The thinking exhaustion retry path pushes a recovery message into `session.messages` and calls `continue` to re-enter the main `loop`. However, the `FinalResponse` has already been received, so the stream is complete. On the next iteration, `stream.next()` returns `None`, hitting the `None` arm at line 1867, which breaks the loop — no retry actually happens. Compare with PROV-040 (truncation recovery) which correctly creates a new stream.

2. **`downgrade_thinking_level()` is never called in stream loop integration — thinking budget never actually reduced** (stream_loop.rs:1603-1635)
   The recovery message claims "Your thinking budget has been reduced" but no code modifies the agent's thinking configuration. `downgrade_thinking_level()` exists (line 397) and is tested, but is dead code in the production retry path.

3. **Session-level progressive degradation (Rule[7], Scenario 8) is not implemented** (stream_loop.rs — no cross-turn counter)
   No session-level thinking exhaustion counter persists across turns. Session struct has no field for this. The test simulates it with local variables but doesn't test any production code path.

4. **Context preservation at >90% utilization (Rule[5], Scenario 7) is not implemented** (stream_loop.rs — no 90% check)
   No context window utilization check, no session archive persistence before retry. The test only verifies threshold logic with local variables.

## 🟡 Warnings (Should Fix)

1. **Recovery message captures `assistant_text` as "reasoning" but this is output content, not thinking content** (stream_loop.rs:1605-1608)
   The code uses `assistant_text` (visible output) as `captured_reasoning`, but architecture notes say the reasoning field should be preserved. The actual thinking delta content is streamed via ReasoningDelta events and is NOT accumulated in `assistant_text`.

2. **Tests for Scenarios 7 & 8 don't test production code paths** (tests lines 326, 368)
   Both tests use local variables to simulate behavior rather than calling production functions. They verify building blocks but not integration logic.

3. **Rule[7] specifies 6-level degradation (XHigh→...→None) but code only has 4 levels (High→Medium→Low→Off)**
   Architecture note[0] mentions 6-level enum but `ThinkingLevel` has only 4 variants. Feature scenarios match the 4-level implementation, but the rule doesn't.

## 🟢 Observations (Nice to Have)

1. Architecture note[3] mentions "budget clamping as primary prevention" — not implemented
2. Architecture note[5] says "For Adaptive models, inject a system hint" — no special handling for Adaptive models

## Coverage Verification
- Feature file: spec/features/thinking-token-exhaustion-recovery-*.feature — OK (present, correct structure)
- Test file: codelet/cli/tests/thinking_exhaustion_recovery_test.rs — ISSUE: Scenarios 7 & 8 test local variables not production code
- Impl file: codelet/cli/src/interactive/stream_loop.rs — ISSUE: integration missing (retry, downgrade, cross-turn, context preservation)
- Scenario coverage: 9/9 linked but 4 scenarios have hollow implementations
