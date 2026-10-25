# BUG-265 source/spec/test preparation checkpoint

Historical preparation checkpoint. The later controller clearance, actual
runtime RED, current native status and remaining work are recorded in
`runtime-red-checkpoint.md`; use that newer checkpoint for continuation.

## Phase and resource boundary

Preparation only. No new Cargo build, test executable, runtime test, mechanical
production move or accounting repair was run in this phase. No cache was deleted
and no cleanup approval was requested. The controller must explicitly clear the
build phase after resolving the old-cache decision. `rust/target` is preserved.
The last `df -h .` observation in this phase was 7.1 GiB available; the strict
floor remains 6 GiB. This is not sufficient permission to start compilation.

BUG-265 remains **specifying**, estimated at 5. The approved split ledger remains
`module-plan.md`. Both `rust/sessions/src/background_session.rs` and
`rust/agent-loop/src/agent_loop.rs` are untouched by this preparation.

## Prepared artifacts, not execution evidence

- Native Example Mapping: three rules and seven examples, covering real-input
  reset boundaries, goal lifetime and CONT-008/009 invariants.
- `spec/features/recovery-completion-accounting.feature`: seven scenarios,
  tagged `@wip`, `@BUG-265`, `@session`, `@completion`.
- `rust/agent-loop/tests/bug265_recovery_completion_accounting.rs`: seven
  scenario tests; native coverage mappings validate all matching step comments.
- `rust/agent-loop/tests/support/bug265.rs` and `bug265_wire.rs`: real-loop
  fixture composition and synthetic wire responses. All three new Rust files
  are below 300 lines (176, 197, 76 at this checkpoint).
- The `.feature.coverage` sidecar maps test source only, with no implementation
  mappings. Mapping statistics are **not** runtime coverage or test success.

The tests reuse `support/automatic_compaction.rs` and its real filesystem,
BackgroundSession and admitted loopback OpenAI path. They do not substitute an
isolated accounting model or mutate counters to manufacture consumed nudges.
Actual clean stops consume nudges; real repetitive SSE output invokes the loop
detector; overflow invokes the existing compaction/retry path. Native rejected
Done calls use `work_unit_id: NONE` under the existing auxiliary admission scope.
Tests capture actual request tool registration, registry counts and retry context.
The goal tests drive the synthetic fixture's request-ID-checked pause response;
this is test source, not a real human answer or controller approval.

The replacement/clear control first synchronizes actual native rejections through
a clean-final settlement, ensuring nonzero inner and registry counts before the
clear. An injected non-overflow provider failure ends each user task without
accepting Done or escalating a deliberately unfinished goal. The configured
replacement verification command must not run: rejected Done arguments fail
Tier 1 first. No actual provider or credential request occurred in this phase.

## Checks actually performed

- Native `format` of this feature: one file formatted.
- Native `validate` of this feature: valid, exit code 0.
- Native `validate-tags` of this feature: one valid, zero invalid.
- Seven native `link-coverage` calls: all successful, without validation skips.
  The initial call before sidecar creation failed for a missing sidecar and made
  no mapping. A scoped sidecar was then created; no repository-wide generation
  was run and no unrelated coverage file was rewritten.
- `rustfmt --edition 2021 --check --color never` on the new test entry point
  (including its child modules): passed after formatting-only source edits.
- `git diff --check`: passed.
- Scoped `git diff --numstat` for both production files named above: empty.

**Not performed:** compilation, genuine causal RED, green, Clippy, final runtime
coverage, post-split controls, final quality review or native Done transition.
Rustfmt parses source but does not type-check it. No passing runtime result is
claimed for these new tests. Source-based expectations are five defect-sensitive
scenarios and two positive boundary controls; these are not observed outcomes.

## Required continuation after explicit controller clearance

1. Inspect available disk again. Preserve the useful project target directory.
   Do not infer cache deletion authorization from this checkpoint.
2. Capture the missing pre-move CONT-008 runtime baseline with the existing
   `codelet-fspec-tui` integration target `cont008_goal_back_sync_test`, selecting
   these nonbuilding behavioral tests individually if needed:
   - `goal_satisfied_snapshot_writes_the_chrome_goal_state_back`
   - `a_goal_replaced_mid_turn_survives_the_goal_satisfied_write_back`
   - `a_satisfied_goal_is_not_resurrected_on_the_next_dispatched_user_message`
   - `the_dispatch_sync_never_reapplies_a_chrome_goal_the_engine_already_consumed`
   The target itself is oversized (824 lines). Do not append tests there.
3. Under the native testing phase, compile and run the prepared target against
   untouched production source. Fix fixture/type errors before recording causal
   RED. Every claimed failure must expose accounting, not routing/admission or
   missing terminal evidence. Preserve exact command output and exit status.
4. Perform only the approved full mechanical split, retaining every field,
   default, ordering, channel, visibility and original source-shape condition.
   Split oversized source-shape tests into bounded children that read the moved
   code. Split-induced failures are introduced failures, not baseline exceptions.
5. Re-run runtime controls after extraction and demonstrate BUG-265 RED still
   exposes the unchanged defect before any production accounting repair.
6. Implement the smallest causal repair. Preserve goal generations and arming
   before output/provider construction. Re-run the seven regressions and the
   affected controls on final source; only then record green/review/Done.

The intended new regression invocation, **not executed in this phase**, from
`rust/` is:

```sh
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug265_recovery_completion_accounting -j 2
```

CONT-008 controls use the same offline/profile/jobs bounds with package
`codelet-fspec-tui` and target `cont008_goal_back_sync_test`. Existing baseline
evidence in `baseline-runtime-controls.log` contains 6 CONT-009 + 4 paused delivery
+ 2 history-clear broadcast + 7 admission passes (19 total), completed before
this preparation. It does not cover CONT-008 or BUG-265 RED.

`background_session_shape` contains nested default-profile Cargo invocations.
Do not run the broad target. Select genuinely nonbuilding assertions only after
inspection, and disclose excluded scenarios rather than marking them passed.
No denied discovery pipeline or rejected compaction-summary operation was retried.
