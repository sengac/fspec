# BUG-221: Thinking-only responses fall between recovery and continuation

## Scope and confidence

Investigated on 2026-10-21 at `7e60de8e`. The screenshot matches the persisted response exactly. A conditional recovery gap is confirmed in source. The incident's actual wire finish reason was not retained in the inspected data, so token-limit termination remains a strong inference, not a proven runtime fact.

No code changes, Cargo tests or provider calls were performed during the audit.

## Screenshot and durable evidence

The 13:11 local-time screenshot shows Qwen3.8-flash-next-iq3_s under Strada, with `auto-continue (0/300)` and thinking ending `priority is: (a) correct guard, (b) idempotent`.

- Session: `dc428d30-6e5e-4225-bfe4-c09f6b252859`.
- Persisted assistant message at 03:10:45.214238 UTC: 4,096 thinking blocks, no ordinary text or tools, with the matching ending. Local record: `~/.fspec/messages/messages.jsonl:229806`.
- Matching manifest accounting: 93,805 input, 4,096 output, 97,901 total, and absent numerical `reasoning_tokens`. Local manifest: `~/.fspec/sessions/dc428d30-6e5e-4225-bfe4-c09f6b252859.json:543`.
- Rig reports the matching multi-turn stream finished at 03:10:45.201730; the session becomes Idle at 03:10:46.471992. Combined-log anchors: `~/.fspec/logs/fspec-combined.log.2026-10-21:1323` and line 1324.

Do not copy the full private transcript into fixtures. A small synthetic thinking-only response can preserve the failing shape.

## Source-confirmed gap

1. `rust/cli/src/interactive/stream_loop.rs:1337` maps missing numerical reasoning usage to zero before detection, despite captured reasoning deltas.
2. `rust/cli/src/interactive/recovery_thinking.rs:39` requires a length-related stop, a positive reasoning count and output usage below a threshold. The observed usage cannot pass: reasoning is absent, and reported output is 4,096 despite no visible answer.
3. `rust/patches/rig-core/src/providers/openai/completion/streaming.rs:308` normalizes a wire `length` finish to `max_tokens`.
4. `rust/cli/src/interactive/auto_continue.rs:74` excludes `length`/`max_tokens`; only `stop`, `end_turn` or no stop reason may nudge.

Consequently, a thinking-only length final can trigger neither exhaustion recovery nor ordinary auto-continue. The code gap is certain; whether this exact final had a length reason is not.

Numerical reasoning usage is optional across providers. Anthropic normalization does not populate it (`rust/patches/rig-core/src/providers/anthropic/streaming.rs:85`); Gemini and custom adapters also depend on optional provider counters. Visible reasoning content and billing output are not interchangeable with a dedicated reasoning count.

## Important distinctions

- `0/300` is outstanding zero-progress nudges, not total API requests. Tool activity can refund a nudge.
- Accepted done/goal completion, user interruption, stalls and terminal errors intentionally stop without nudging.
- An outer EOF branch saves partial text and emits Done without the decision, but raw provider EOF is not necessarily that branch: Rig normally synthesizes a final. Do not claim EOF caused this incident without tracing the actual event sequence.
- The 03:49:37 compaction admission failure is later and independently tracked in BUG-215.

## Existing coverage and regression plan

- Review `spec/features/thinking-token-exhaustion-recovery-detect-budget-depletion-and-preserve-context-across-all-providers.feature` and `spec/features/auto-continue-engine.feature`.
- `rust/cli/tests/thinking_exhaustion_recovery_test.rs:74` supplies positive reasoning usage with zero output; it misses the captured shape.
- Some wiring assertions in `rust/cli/tests/auto_continue_engine_test.rs:256` inspect source rather than execute streaming finalization.
- Add a streamed thinking-only final with absent reasoning count and a length stop; verify bounded recovery instead of silent Idle at 0/300.
- Test mixed text/thinking, total output including reasoning, genuinely useful truncated output, absent finish reason, normal clean completion, errors and user cancellation.
- Preserve bounded retries and honest token accounting; do not fabricate numerical usage or unconditionally retry unsafe stop reasons.

BUG-222 covers whether a successful exhaustion detection actually changes the next request. BUG-223/BUG-224 cover separate Gemini bypasses; BUG-225 covers whether the indicator reflects backend settings.
