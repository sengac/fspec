# BUG-222: Thinking-recovery retry does not apply its claimed reduction

## Scope and confidence

Source-confirmed conditional defect at `7e60de8e`, researched on 2026-10-21. This applies after exhaustion detection succeeds. It does not explain why the second screenshot's missing reasoning counter prevented detection (BUG-221). No new runtime reproduction or test execution was performed.

## Source chain

- `rust/cli/src/interactive/recovery_thinking.rs:75` builds a recovery prompt claiming that the thinking budget has been reduced.
- `rust/cli/src/interactive/stream_loop.rs:1365` tracks cross-turn exhaustion and may downgrade `session.session_thinking_level` after a threshold.
- The immediate retry emits `Retrying with reduced reasoning budget...`, constructs fresh token state/hook, and calls the existing agent's `prompt_streaming_with_history_and_hook` at line 1444.
- It does not reconstruct the agent with a lower reasoning level or increased output allowance. Fresh bookkeeping is not a change to the request's model settings.

The injected prompt does request more concise reasoning, but that is distinct from the explicitly claimed configuration change. A session-level downgrade may affect a subsequently constructed agent, not necessarily this immediate retry.

## User impact

Retries can repeat the same resource constraint while telling the operator and model that the constraint changed. This wastes the bounded retry budget and makes recovery diagnostics unreliable. The problem is independent of whether the exhaustion heuristic is expanded.

## Existing contract and test gaps

- `spec/features/thinking-token-exhaustion-recovery-detect-budget-depletion-and-preserve-context-across-all-providers.feature` describes exhaustion recovery and progressive degradation.
- `rust/cli/tests/thinking_exhaustion_recovery_test.rs` exercises detection/message helpers; inspect coverage before adding duplicate pure-function assertions.
- `rust/agent-loop/tests/rpc085_thinking_config.rs` is relevant to propagation of effective thinking settings into agent construction.

## Regression plan

1. Drive a real streamed exhaustion response into recovery using a local provider fixture.
2. Capture the first and retry requests, including reasoning configuration and output allowance.
3. Assert that an announced reduction produces a supported, effective setting change; if a provider cannot reduce it, assert an honest alternative status and bounded policy.
4. Cover already-minimal reasoning, absent reasoning control, hard model caps and cross-turn threshold downgrade.
5. Retain captured reasoning and conversation history without duplicating prompts or breaking tool pairs.
6. Assert interrupt, cancellation and exhausted recovery budgets never start an extra request.

## Recommended direction and questions

Choose and specify the actual corrective policy: rebuild with reduced reasoning, increase a safe output allowance within capacity, or make a prompt-only retry with truthful wording. Respect provider-specific thinking modes and admission limits; do not globally increase token budgets or claim an unsupported configuration change.

Related cards: BUG-221 (detection), BUG-215 (admission limits). The retry must use the same effective limits that admission validates.
