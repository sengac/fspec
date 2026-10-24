# BUG-223 — Gemini continuation bypasses shared completion settlement

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; read-only source investigation with this research attachment as the only assigned output.
- **Source-confirmed conditional defect; high confidence:** when the Gemini empty-response helper completes normally, it emits Done and returns before the caller's shared completion-contract settlement.
- This establishes a reachable control-flow gap, not an executed reproduction or an explanation of a particular incident. No tests, builds, or endpoint requests were executed.

## Source chain
1. `rust/cli/src/interactive/gemini_continuation.rs:79` restricts the helper to provider `gemini`; `:83` additionally gates on the turn-completion facade's model check. `:88` requires a `FullLoop` continuation strategy.
2. `rust/cli/src/interactive/gemini_continuation.rs:100` records the empty-after-tool continuation and `:105` appends its user prompt. `:149` opens the continuation stream.
3. `rust/cli/src/interactive/gemini_continuation.rs:288` handles its FinalResponse; `:296` can restart another empty continuation. On ordinary completion, `:368` saves text and updates token accounting.
4. `rust/cli/src/interactive/gemini_continuation.rs:388` clears the progress callback, `:389` emits Done with the stop reason, and `:390` returns `GeminiContinuationResult::Completed`.
5. `rust/cli/src/interactive/stream_loop.rs:1274` invokes the helper; `:1291` handles Completed by immediately returning `Ok(())`.
6. The skipped shared path starts at `rust/cli/src/interactive/stream_loop.rs:1552`: `:1567` consumes done acceptance, `:1576` settles refund accounting, and `:1599` selects goal versus auto-continue decisions.
7. `rust/cli/src/interactive/stream_loop.rs:1636` invokes shared accepted-done teardown; `:1640` handles budget warnings, `:1654` escalation, and `:1667` counted nudges. The helper's normal completion never reaches these sites.

## Impact and boundaries
- An armed session can receive a normal Gemini helper completion without the clean-stop nudge or goal decision that the main loop would make.
- A done acceptance recorded inside this helper lacks the main loop's shared settlement/teardown path; summary, goal clearing, and counter consistency need explicit regressions.
- Token bookkeeping in the helper does not establish completion-contract correctness. The bypass also precedes the main thinking-exhaustion block at `rust/cli/src/interactive/stream_loop.rs:1333`, but that is separate from proving the completion gap.
- This is Gemini-specific. Do not apply it to a built-in OpenAI screenshot or assume every Gemini model enters the helper.

## Reproduction outline — proposed, not executed
1. Use an offline scripted Gemini stream and a model/history combination satisfying the helper's facade gates; enable continue or set a goal.
2. Produce tool activity followed by empty assistant output so the helper opens its continuation stream.
3. Complete that continuation with nonempty assistant text and a clean end-turn, without accepted done().
4. Capture requests, Done events, nudges, and session counters. The source predicts helper Done and return instead of the shared nudge decision.
5. Repeat with accepted done(), exhausted budget, goal escalation, and compaction/interruption controls; compare against the ordinary main-loop path.

## Existing specifications/tests and missing regressions
- `spec/features/auto-continue-engine.feature:35` requires a counted nudge for an armed stop without done(); `:41` requires accepted-done summary settlement; `:87` identifies the clean FinalResponse settle point.
- `rust/cli/tests/auto_continue_engine_test.rs:93` tests the decision helper; `:125` covers accepted completion. The source-shape assertion at `:257` reads only `stream_loop.rs`, so it cannot establish helper-path parity.
- `spec/features/handle-gemini-continuation-compaction-gracefully.feature:32` covers helper compaction recovery, not normal completion-contract settlement.
- `rust/cli/tests/cmpct042_gemini_continuation_token_state_test.rs:1` is related token-seeding coverage, not an end-to-end completion-contract regression.
- Missing: behavioral helper-path tests for counted nudge, budget exhaustion, accepted summary/goal teardown exactly once, refunds after helper tool activity, and no nudge on interruption/error/compaction.

## Recommended direction
- Return a structured continuation outcome to the common settlement owner, or extract a shared settlement operation that both paths invoke; avoid a second independently maintained completion contract.
- Carry the continuation's actual stop reason, usage, tool activity, and acceptance into that operation. Preserve thought-signature preparation, compaction policies, token accounting, and single terminal emission.
- Specify and write failing offline regressions first; keep existing main-loop and helper compaction behavior covered.

## Uncertainties and related cards
- Runtime frequency and incident attribution are unmeasured. The facade gates and actual response sequence determine exposure; no private logs/configuration are copied here.
- BUG-224 is a separate Gemini parser-terminal problem that can prevent FinalResponse entirely. BUG-221 concerns thinking exhaustion; BUG-222 concerns unchanged retries, not this proven early return.
- CONT-002, CONT-003, CONT-005/CONT-006, and CONT-007 own the bypassed completion/goal/counter contracts; CMPCT-002 and CMPCT-042 cover adjacent recovery/accounting invariants. CONT-010 concerns lifecycle semantics, not proof that this helper settles correctly.
