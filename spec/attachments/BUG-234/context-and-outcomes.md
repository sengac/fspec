# BUG-234: Compaction deadline exhaustion

Research date: 2026-10-22. HITL requested investigation before card creation, then
approved this focused follow-up after timeline review. No exact stalled phase or
production fix has been established. Source references are triage snapshots.

## Context and impact

Two compaction operations failed on the same session with `compaction deadline exceeded`.
A finite deadline is a required safeguard, not itself a defect. The investigation must
determine how exploration, synthesis, correction, and cleanup consume the operation
budget and make failures understandable without bypassing validation.

## Incident timeline

Session: `7a6e21ef-f5cd-4155-826d-8648571b04fe`. All times are UTC.

| Operation | Entered Compacting | Failure surfaced | Evidence |
|---|---|---|---|
| First | Oct 21 23:56:53.858792 | Oct 22 00:07:17.227397 | Oct 21 log `:787`; Oct 22 log `:80` |
| Second | Oct 22 00:08:52.567715 | Oct 22 00:18:52.600730 | Oct 22 log `:87`, `:214` |

Logs are `~/.fspec/logs/fspec-combined.log.2026-10-21` and
`~/.fspec/logs/fspec-combined.log.2026-10-22`.

- The second interval is essentially exactly 600 seconds, matching the default budget.
- The first failure surfaces about 10 minutes 23 seconds after the Compacting transition;
  its Idle transition occurs roughly 23 seconds before the final error.
- Numerous `SessionSearch` originals reads occur in both windows.
- Correction warnings at 00:03:37 and 00:15:23 have no operation/session fields. Their
  proximity is not proof that they belong to either timed-out builder.
- Lifecycle timestamps are observations, not necessarily the precise instant the
  internal deadline was initialized. Incident-specific overrides remain unknown.

## Code and existing protections

- `rust/cli/src/compaction_operation.rs:61` initializes a shared operation deadline;
  `rust/cli/src/compaction_operation.rs:73` enforces it with `timeout_at`.
- `rust/cli/src/compaction_operation.rs:159` bounds candidate correction attempts.
- `rust/cli/src/compactor_sub_agent.rs:133` constructs the operation with the configured budget.
- `rust/cli/src/interactive/recovery_stall.rs:26` defines the 600-second default;
  `rust/cli/src/interactive/recovery_stall.rs:87` resolves the effective timeout.
- `rust/agent-loop/src/compaction_builder.rs:96` separately bounds builder execution;
  its inner deadline error has different wording.
- `spec/features/compaction-builder-budget-recovery.feature:1` and
  `spec/features/compactor-occupancy-runway.feature:1` describe existing synthesis/runway contracts.

Related completed work: CMPCT-057, CMPCT-079, BUG-211, BUG-229, and BUG-214.
Verify the incident executable revision before assuming these fixes were active.
BUG-232 owns malformed output; BUG-235 owns cancellation-state disagreement.

## Investigation questions

1. Which provider/model, effective timeout, operation, and attempt belong to each run?
2. How much time was spent reading evidence, awaiting provider responses, synthesizing,
   correcting candidates, cancelling work, and publishing lifecycle state?
3. Is existing exploration/synthesis runway enforced on the failing route?
4. Why did the first run publish Idle before its final deadline error, and could a user
   start conflicting work during that interval?
5. Are late builder results discarded after cancellation or deadline expiry?

## Expected outcomes

- Produce a correlated timeline that explains budget consumption and the first run's
  status/error delay; explicitly identify anything the available evidence cannot prove.
- Ensure exploration yields according to existing budget/runway policy so synthesis and
  bounded correction are not unintentionally starved on supported routes.
- Keep one effective operation deadline rather than resetting it on every retry.
- Report operation/session/provider identity, phase, effective budget, and retryability
  without requiring full conversation payload logging.
- Retain the committed boundary on failure, reject late installs, and preserve the
  established distinction between fresh-input retry and automatic/background continuation.

## Regression evidence and completion

Use deterministic timing and controlled provider responses rather than a real ten-minute
sleep. Cover slow exploration, stalled synthesis, invalid-output correction, deadline
expiry, cancellation, late output, and the allowed explicit retry path.

Review `rust/cli/tests/cmpct057_operation_lifecycle.rs:1`,
`rust/agent-loop/tests/bug211_compaction_budget_recovery.rs:1`,
`rust/agent-loop/tests/bug229_compactor_runway.rs:1`, and
`rust/cli/tests/bug214_compaction_cancellation.rs:1` before adding tests.

Do not merely increase the timeout, fabricate a fallback summary, relax validation,
or force ordinary inference to resume. Completion requires an explained/reproduced
failure mode, scenario-linked regression evidence, and scoped quality checks.

## 2026-10-22 implementation investigation

Confirmed supported-route gap: `compaction_runway::Runway` checks occupancy and
`compaction_execution::run` checks depth, but neither originally limits elapsed
exploration time. A slow provider can consume the whole outer deadline before
depth or occupancy yields, leaving no synthesis opportunity. The deterministic
regression uses a real local HTTP provider: retrieve both captured originals,
stall the following exploration response, and require tool-free synthesis inside
a 900 ms operation budget. It exercises registered recovery and generation.

Timeline recheck (UTC, same original log files):

- First operation: Compacting 23:56:53.858792; Idle 00:06:53.855154; final error
  00:07:17.227397. Idle occurs 599.996362 seconds after Compacting; final error is
  23.372243 seconds after Idle. The operation itself need not have run 623 seconds.
- Second operation: Compacting 00:08:52.567715; Idle 00:18:52.599951; final error
  00:18:52.600730 (600.033015 seconds after Compacting).
- Originals reads targeting the same session continue until 00:06:50.328682 in
  the first window. They establish evidence-reading activity close to expiry,
  not exact provider-wait or synthesis durations.
- Current `BackgroundOutput::emit(CompactionFailed)` emits Idle before returning
  the error to the outer loop. Outer-loop reconciliation/handler cleanup and
  terminal-recovery classification precede its final error log. This explains
  why those timestamps may differ; available logs do not identify which step
  consumed the first run's 23.37 seconds. No unsupported race claim is made.
- The old correction warnings have no operation identity. They cannot reliably
  identify either attempt. The incident executable revision, effective override,
  exact provider/model, per-phase durations and allocation identities remain
  unproven by these correlated events alone.

Implementation coordination scope: this card changes the builder deadline wrapper,
the shared execution's elapsed-time yield, and adds an inherited timing helper.
The BUG-232/233 candidate-validation and correction prompt semantics remain owned
by the DAG worker. Validators, durable publication and interruption safeguards
must remain unchanged. Final RED/GREEN and scoped check results follow below.

### Implementation and regression evidence

- New `compaction_timing` task-local context inherits the absolute operation
  deadline from CLI recovery or initializes it once for direct generation.
  Corrections divide the remaining time among remaining candidate slots;
  exploration yields after half the current slot, leaving tool-free synthesis
  time. Candidate fractions are soft exploration estimates; synthesis can borrow
  unused operation time. The production 600-second default
  is unchanged. Existing validation and guarded publication are not relaxed.
- Builder/session/provider/model spans and operation/attempt/phase budget and
  elapsed-time events now correlate progress without conversation payloads.
  Synthesis timeout identifies the phase and explicit fresh-input retry policy;
  operation failure logs retryability while blocking automatic continuation.
- Behavioral RED: `/tmp/bug234-red-behavioral.log`, 1 passed / 2 failed. Slow
  exploration reached `compaction deadline exceeded`; stalled synthesis returned
  an unqualified builder deadline. Both fail the new acceptance assertions.
- Early fixture failures were investigated, not accepted as RED evidence:
  `bug210_completion_metrics` assumes a byte-style padding token count and failed
  11371 vs 90336 before reaching the provider. The replacement composes the
  existing terminal fixture with shared temp-directory helpers, persists actual
  structured original envelopes, and matches the selected provider identity.
  The initial replacement also needed explicit `work_unit_id: NONE` on its mock
  SessionSearch calls. These are test-fixture corrections, not production fixes.
- One debugging run accidentally retained the prior generation's default
  operation duration because its test override was applied only afterward.
  Applying the test budget before that first operation restores the intended
  900 ms budget. Tests also have outer guards against unexpectedly long runs.
- GREEN: `/tmp/bug234-green-verified.log`, 3/3 passed in 1.98 seconds. Both
  registered recovery and generation synthesize with both original envelopes;
  stalled synthesis retains the prior durable generation and emits no completion;
  paused-clock correction verifies identical inherited absolute deadlines.
- `/tmp/bug234-235-regressions.log` passes 16 CLI cancellation/ownership/lifecycle/
  validation tests plus 3 of 4 stream recovery tests. The remaining adjacent
  admission-size fixture expects rejection but receives success (`builder failure
  must stop recovery`); it is not weakened or changed under this timing card.
- Scoped CLI/agent-loop check and library clippy pass in
  `/tmp/bug234-235-final-check.log` and `/tmp/bug234-235-final-clippy.log`.
  Feature validation, registered tags, and coverage audit pass with all three
  scenarios linked 1:1 to tests. Final strengthened lifecycle assertions and
  repeated quality checks are recorded in `/tmp/bug234-acceptance-final.log`,
  `/tmp/bug234-235-acceptance-check.log`, and
  `/tmp/bug234-235-acceptance-clippy.log`.

Initial acceptance run passes 3/3 in 1.95 seconds, including the strengthened
assertions that synthesis failure leaves inference blocked, clears busy state,
returns the session to Idle, and preserves the prior committed generation.
Repeated CLI/agent-loop check and library clippy both pass after the final
production changes. `git diff --check` is clean. All new source/test/feature/
evidence files remain below 300 lines. No commits or branches were created.

### Reopened review: partial batches and borrowable synthesis time

The initial three-scenario milestone above was superseded by parent review.
BUG-234 remains implementing until parent acceptance of the expanded contract.
Two feature scenarios were added before runtime regressions and production edits.
Runtime RED: `/tmp/bug234-review-red-isolated.log`, 0 passed / 2 failed:
completed originals were missing at handoff and synthesis stopped at a candidate
fraction despite unused absolute operation time.

- Both rig prompt-hook traits now expose a backward-compatible default-no-op
  typed completed-tool observer. Runway captures actual call/result pairs at the
  result boundary, not unfinished tool scaffolding or invented results.
- Runway caches the prepared tool-free synthesis request before tools execute,
  then replaces only its history at handoff. Rebuilding via `agent.completion()`
  previously waited behind a slow Bash call in the ToolServer actor. Final
  complete-request admission still runs over the actual gathered history.
- Synthesis uses the inherited absolute operation deadline, not the candidate
  exploration estimate; corrections never reset that absolute deadline.
- GREEN: `/tmp/bug234-review-green.log`, 5/5 in 7.93 seconds. Partial-batch coverage
  exercises registered recovery and generation nonstream factories, plus the
  real streaming executor with an ordered local OpenAI Responses SSE boundary
  and provider selection `zai` (not full ZAI factory integration). The Chat
  Completions fixture was unsuitable because its adapter emits HashMap tool
  order, so Bash could precede originals despite supplied tool indices.
- Borrowing coverage accepts one or two exploration requests: occupancy may
  hand off early. It still requires exactly one final synthesis, elapsed time
  beyond the candidate estimate, and durable publication before absolute expiry.
- `/tmp/bug234-review-cli-regressions.log`: 24/24 cancellation, ownership,
  lifecycle, validation and recovery tests pass. Scoped CLI/agent-loop/rig check
  and library clippy pass in `/tmp/bug234-review-check.log` and
  `/tmp/bug234-review-clippy.log`. Five scenarios are linked 1:1 to tests and
  implementation, with a passing coverage audit. Coverage's stale cached total
  was corrected from three to five after fspec retained the old count.

### Adjacent test hang investigation after user interruption

The broader adjacent run was interrupted after the user observed roughly 3000
seconds of silence. `/tmp/bug234-review-adjacent.log` stopped at
`scenario_codex_physical_requests_respect_the_exact_incident_boundary`.
No test/cargo process remained after interruption. A fresh standalone reproduction
used an independent 12-second OS kill guard and was explicitly stopped after a
two-second macOS sample, rather than rerun the unbounded test to completion.

Confirmed chain: `bug229_codex_runway.rs:140-144` constructs about 922 KB of one
unbroken `x` run, sizing it by serialized bytes as if bytes were token units.
Current request admission instead calls real cl100k tokenization at
`providers/src/request_admission.rs:159`. The test's `stream.next()` at line 161
executes that synchronous CPU work on the Tokio test thread. CPU was 88.8% at
two seconds; all 954 samples of that active thread were inside admission ->
token estimator -> tiktoken `byte_pair_encode` / `_byte_pair_merge`.
The dependency repeatedly scans remaining merge ranks and removes a Vec element,
giving pathological quadratic behavior on this giant single regex piece.
This is not a network wait or mutex deadlock. Raw evidence:
`/tmp/bug234-hang-sample.txt`, `/tmp/bug234-hang-repro.log`.

The test has neither an operation deadline nor a test-level wall-clock guard.
Even adding a Tokio timeout alone cannot preempt synchronous tokenizer work
within one poll. The shell tool's requested timeout did not bound the original
run; subsequent reproduction therefore used an independent OS guard.
Separately, `bug210_completion_metrics.rs:110` pads about 90 KB by byte arithmetic
and expects 90336 tokens but now measures 11371. That explains the earlier four
fixture failures in `/tmp/bug234-review-agent-regressions.log` (4 passed / 4
failed), and synthesis-admission's one fixture failure (1 passed / 1 failed).
These byte-count assumptions need token-aware fixtures with bounded, separated
input. The non-yielding admission path also warrants separate production
performance/deadline coverage. Neither validators nor admission were weakened,
and these adjacent fixtures were not silently repaired under BUG-234.

### Final completion review

The expanded contract now has six scenarios, each mapped to one integration test.
The sixth rejects candidates whose operation expires during selected admission or
before deferred generation application. It also proves that expiry after promotion
does not undo an authoritative generation, and ordinary InjectSummary submissions
do not accidentally inherit a stale generation deadline.

- Fresh acceptance exposed timing-fixture sensitivity: the 900 ms operation gave
  first-attempt exploration only 150 ms, including setup. Diagnostic requests in
  `/tmp/bug234-completion-diagnostic.log` prove one failing borrowing run sent zero
  exploration requests. Test budgets are now three seconds with 500 ms initial
  exploration and 1200 ms synthesis, still exceeding the one-second candidate
  estimate. All evidence, request-count and publication assertions remain, and
  borrowing now explicitly requires the gathered originals. Production's default
  timeout is unchanged; the exact source of test setup latency is not asserted.
- Scaffold RED: `/tmp/bug234-scaffold-red.log` loses accompanying assistant text.
  The completed-tool observer now receives the actual assistant message. Runway
  preserves its text, reasoning, signatures and completed-call grouping, filtering
  only unfinished calls without inventing results. Local Anthropic SSE additionally
  verifies signed reasoning remains before text and the authentic completed call.
- Publication RED: `/tmp/bug234-publication-red.log` reproduces expired admission
  and deferred candidates being promoted. CLI admission now checks the inherited
  absolute deadline; direct generation carries that deadline into its pending
  candidate and checks it at guarded durable promotion. Cancellation and validators
  remain enforced, while already-promoted generations remain authoritative.
- Default-stack regressions during these additions were diagnosed using macOS crash
  reports: normal nested builder/agent future polling exhausted the test thread
  stack, not recursive calls. Exploration and provider-dispatch futures are now
  heap-pinned at their boundaries. Acceptance passes with the default thread stack;
  no permanent stack-size override or new Tokio runtime was introduced.
- GREEN: `/tmp/bug234-completion-six-scenarios.log` passes all six scenarios (five
  elapsed-runway tests plus one publication-deadline test). Partial batches cover
  registered recovery, direct generation, the ordered Responses streaming executor,
  and signed Anthropic streaming. All provider traffic uses local test servers.
- Adjacent checks pass: 24 CLI lifecycle/cancellation/validation/recovery tests in
  `/tmp/bug234-completion-cli-regressions.log`, two provider history tests in
  `/tmp/bug234-completion-provider-history.log`, and three Codex reload tests in
  `/tmp/bug234-completion-codex-reload.log`. Scoped provider/CLI/sessions/agent-loop/
  rig library check and clippy pass in `/tmp/bug234-completion-check.log` and
  `/tmp/bug234-completion-clippy.log`. Commands have independent OS-level limits.
- This feature passes syntax, tag and coverage validation. Repository-wide tag
  validation reports 526 unrelated existing invalid features; those are not changed.
  The documented pathological tokenizer fixture is deliberately not rerun. Incident
  revision, exact provider phases and the original cleanup delay remain unproven
  beyond the evidence and timeline already recorded above.

Final repeated acceptance: `/tmp/bug234-completion-verified.log` passes 6/6 with
the default stack. Scoped formatting (Rust 2021 workspace and Rust 2024 rig patch),
both staged/unstaged whitespace checks, and the 19-reference coverage audit pass.
Coverage records six scenarios, six unique test mappings, and 100% traceability.
