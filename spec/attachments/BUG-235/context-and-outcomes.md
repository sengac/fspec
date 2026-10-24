# BUG-235: Cancellation intent and compaction state disagreement

Research date: 2026-10-22. HITL-approved state investigation, separate from deadline
exhaustion. The state-identity/race explanation is a hypothesis, not an established
root cause. Source references describe the triage snapshot.

## Context and impact

The compaction hook reported a threshold trigger, but recovery immediately observed
`compaction_needed=false`. Defensive recovery still entered compaction. This is a
state-consistency investigation, not proof that cancellation was lost or that the
session continued without compacting.

## Incident sequence

Log: `~/.fspec/logs/fspec-combined.log.2026-10-21:783` through `:788`.
The subsequent lifecycle event identifies session
`7a6e21ef-f5cd-4155-826d-8648571b04fe`.

| UTC time | Observation |
|---|---|
| 23:56:53.857294 | Hook: `COMPACTION TRIGGERED: 190847 tokens > 190000 threshold` |
| 23:56:53.858026 | Streaming layer yields typed `PromptCancelled` |
| 23:56:53.858411 | Classifier: `PromptCancelled without compaction_needed=true; recovering anyway` |
| 23:56:53.858754 | Recovery helper observes `compaction_needed` already true |
| 23:56:53.858792 | Session transitions from Running to Compacting |

The classifier attempts to set the missing flag itself. Therefore the later
already-true warning is explainable and must not be filed as a second independent bug.
The initial hook and cancellation messages lack the same explicit session field as
the lifecycle message; confirm event ownership when reproducing the chain.

## Code and existing contracts

- `rust/core/src/compaction_hook.rs:231` handles threshold-triggered compaction intent.
- `rust/cli/src/interactive/error_classifiers.rs:273` recognizes typed cancellation
  and defensively recovers despite a missing flag.
- `rust/cli/src/interactive/recovery_compaction.rs:1` begins recovery and retains intent.
- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs:1` carries cancellation
  through the streaming boundary.
- `spec/features/replace-stringly-typed-is-compaction-cancelled-with-structural-prompterror-downcast.feature:1`
  describes the typed-error-chain contract.

Related completed work: CMPCT-026 and BUG-144. BUG-234 investigates elapsed-time and
phase budgets; do not silently broaden this card into that separate problem.

## Investigation questions

1. Do the hook, stream, classifier, and recovery helper share the same token-state
   allocation and session/turn generation?
2. Can state replacement, reset, poisoning, or another writer clear intent between
   trigger and recovery?
3. Can an unrelated cancellation/event be mistaken for this threshold trigger?
4. How are user interruption, poisoned-state cancellation, and threshold compaction
   represented and distinguished on each route?

## Expected outcomes

- Reproduce and explain the disagreement, including relevant state/session ownership;
  do not name a race as the cause without evidence.
- Preserve threshold-compaction intent through cancellation and handoff using consistent
  session/turn state and the existing typed cancellation contract.
- Keep defensive recovery for genuine mismatches; do not remove it simply to hide warnings.
- Preserve user interruption semantics and prevent stale cancellation from affecting a
  different session or newer turn.
- Make lifecycle events and diagnostics describe one coherent operation without implying
  that a repaired already-true flag is another failure.

## Regression evidence to produce

Exercise an actual threshold-triggered stream cancellation through classification and
recovery. Also cover nested typed errors, cancellation with a missing flag, user Escape,
state replacement/reset, and independent sessions. Verify no ordinary-inference leak,
duplicate builder start, stuck Compacting state, or cross-session state mutation.

Review `rust/cli/tests/compaction_trigger_reliability_test.rs:1`,
`rust/cli/tests/compaction_cancel_preservation_test.rs:1`, and
`rust/cli/tests/compaction_recovery_unification_test.rs:1` for existing coverage.

## Scope limits and completion

Do not convert every cancellation into compaction, rely solely on error text, disable
the recovery safeguard, or suppress warnings without explaining state ownership.
Completion requires a demonstrated cause or corrected diagnostic attribution, an
agreed intent contract, scenario-linked regression tests, and scoped quality checks.

## 2026-10-22 implementation investigation

Confirmed source-level ownership defect, not a speculative race: the thinking
exhaustion, auto-continue nudge, truncation retry and network retry routes each
created a new `Arc<Mutex<TokenState>>` for the retry hook while error classification
continued reading the outer turn's original allocation. A threshold hook sets
only its own allocation. The resulting typed `PromptCancelled` therefore reaches
the classifier with the outer flag still false. This exactly reproduces the
reported disagreement without concurrent mutation or poisoning.

The incident hook/rig log entries include the auto-continue nudge prompt. The
preceding `CONT-002` nudge count is 3/300 (Oct 21 line 776). This strongly supports
the nudge route, but the old events omit explicit session/turn/state identities,
so the incident cannot prove allocation identity or exclude interleaving with
other sessions. The subsequent already-true recovery message is expected: the
classifier repaired the outer flag. It is now debug-level; genuine missing and
extraneous flag warnings remain unchanged.

The retry hook factory refreshes usage in the existing current-turn allocation
and preserves pending intent. All four affected retry/nudge call sites use it.
It neither changes the existing typed cancellation classifier nor reinterprets
Escape as compaction. Gemini's existing local-to-parent intent forwarding is
unchanged. No new session-global intent map or stale cross-turn state is added.

ACDD evidence: feature `compaction-cancellation-state-ownership.feature` preceded
test `bug235_cancellation_ownership.rs`. Initial RED was the missing shared retry
factory import (E0432, `/tmp/bug235-red.log`); a subsequent test-client type
annotation issue was corrected before GREEN. Tests execute the real rig stream
threshold cancellation, nested typed error classification, independent state
isolation, usage refresh with pending intent, and missing-flag defensive recovery.
Quality checks and final results are recorded below when complete.

Validation so far:

- `/tmp/bug235-green-final.log`: 11/11 passing across
  `bug235_cancellation_ownership`, `compaction_cancel_preservation_test`, and
  `compaction_recovery_unification_test` using `cargo test --profile ci-test -p
  codelet-cli` with explicit test targets.
- `fspec validate`, `validate-tags`, `audit-coverage` pass for this feature;
  `show-coverage` reports all 3 scenarios fully linked to tests/implementation.
- `/tmp/bug234-235-check.log`: scoped CLI/agent-loop `cargo check --profile
  ci-test` passed. First scoped clippy run was blocked by the concurrently edited
  BUG-239 `common/src/logging/privacy.rs` using `expect` in production; that code
  remains owned by its worker. Final shared-tree checks follow after integration.

Final scoped quality checks now pass: `/tmp/bug234-235-final-check.log` and
`/tmp/bug234-235-final-clippy.log` both finish successfully for CLI/agent-loop.
`/tmp/bug234-235-regressions.log` additionally passes all 6 real cancellation
regressions (late candidates, pre-prompt/hook cancellation, admission vetoes,
explicit retry cancellation, and post-promotion preservation), the 3 ownership
tests, all 4 operation lifecycle tests, and all 3 candidate-validation tests.
One adjacent stream-recovery admission-size test fails with `builder failure
must stop recovery`; it does not assert token-state ownership and is recorded
for supervisor integration rather than altered under this card.
