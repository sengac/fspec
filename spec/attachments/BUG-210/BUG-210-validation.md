# BUG-210 implementation validation

## Implemented behavior

- Successful automatic recovery continues the unfinished task only after durable
  commit, context publication, and reconciliation, using the reduced context.
- Automatic continuation preserves cancellation, stops after three recoveries,
  and receives a fresh budget for a genuinely new task. Loop-detector corrective
  retries remain distinct from compaction retries.
- Completion does not announce Idle or final Done before unfinished work resumes.
  Manually applying a summary to an idle session does not start inference.
- Progress updates and clears travel through the session-owned broadcast to the
  Rust TUI without changing focus or another session's progress.
- Completion reports percentage removed and committed coverage. Publication
  precedes completion, and publication failure cannot fabricate success.
- NAPI hooks dispatch through the shared background loop. The former duplicate
  loop is replaced by compatibility exports, not a second recovery policy.

## ACDD evidence

Behavioral regressions were observed before the corresponding fixes. Recovered
logs include premature Idle, exhausted-budget reset, and NAPI sessions failing
to recover through their former loop. The additional loop-detector regression
failed with one inference request instead of two before retry provenance was
distinguished. Its complete eight-test continuation target subsequently passed.

The original implementation validation recorded these passing BUG-210 targets:

| Package | Target | Tests |
| --- | --- | ---: |
| codelet-agent-loop | bug210_automatic_continuation | 8 |
| codelet-agent-loop | bug210_compaction_lifecycle | 2 |
| codelet-agent-loop | bug210_completion_metrics | 5 |
| codelet-fspec-tui | automatic_compaction_progress_bug210 | 3 |
| codelet-napi | bug210_shared_compaction | 1 |

The specifications are separated by integration target to satisfy the workflow's
one-feature/one-test-file rule. This split moves existing scenarios and mappings;
it does not add behavior after implementation.
All 19 BUG-210 scenarios have test and implementation mappings with mandatory
step validation. The five feature coverage audits pass at 100%, with mapped
ranges checked against current files. The replaced terminal-overflow scenario
now lives only in the continuation feature; its legacy coverage entry is removed.

## Additional passing checks

Tests use `cargo test --profile ci-test -p <package> --test <target>`.
No unscoped workspace test run was requested.

- Shared-loop terminal ownership, cancellation cleanup, dependency lifetime,
  bridge-clear failure, committed publication, and token-forwarding targets:
  11 tests pass.
- TUI chunk dispatch, badge auto-hide, percentage display, and single completion
  notice targets: 23 additional tests pass.
- CLI inference handoff, automatic publication, and stream recovery builder
  targets: 10 tests pass.
- Standalone agent-loop, fspec, and TUI dependency guards: 6 tests pass, without
  pulling NAPI into their dependency graphs.
- Provider-session recovery and turn-routing compatibility: 2 tests pass. The
  existing NAPI provider-session helper remains because these tests include it.
- Codex typed-history and session-reload targets requested by the validation
  workflow: 5 tests pass.
- Loop-detector behavioral regressions: 3 tests pass.
- NAPI wrapper smoke target: 36 tests pass. Its architecture target passes 18
  tests, including the changed shared-loop exports and session-hook assertions.
- Scoped Cargo check and Clippy complete successfully, with warnings. Changed
  Rust files pass rustfmt checks; `git diff --check` passes.
- All 1,992 feature files pass Gherkin syntax validation.

## Validation limitations

Broader checks are not fully green; they must not be reported as passing:

- `cmpct044_terminal_overflow_test` does not compile because it treats the
  existing `Result<bool, TerminalRecoveryError>` API as a boolean.
- Older `cmpct045_generate_compaction_behavioral` and CLI cascade-ordering tests
  expect retired fallback behavior, old source locations, or older fixtures.
- The original CLI fixture failures are repaired in the follow-up below.
- NAPI ratio/acknowledgement fixtures now select an explicit offline model,
  check fallible creation, and retain the real continuation input receiver.
- NAPI's remaining architecture assertion expects 66–68 wrapper functions, while
  the existing file has 69. BUG-210 adds no wrapper functions.
- Package-wide formatting also reports unrelated existing formatting differences;
  only changed-file formatting is green.
- Repository-wide tag validation reports 1,466 passing files and 526 files with
  violations. The BUG-210 features use registered component and feature-group
  tags; unrelated tag cleanup is outside this change.

These limitations are retained rather than weakening assertions, restoring
fabricated fallback summaries, changing unrelated fixtures, or claiming a clean
repository-wide test run. No commit is created by this work.
BUG-210 remains in `validating`, rather than being declared done while broader
validation is not green.

## Legacy validation follow-up (2026-10-21)

No production behavior changes in this follow-up. Existing acceptance assertions
remain intact. Test setup no longer depends on ambient provider selection:

- CLI cancellation and retry fixtures use credential-free provider managers.
- CMPCT-057 creates durable originals, selects its model, and uses the persisted
  manifest ID. Its first failure must be the declared builder error, not merely
  any error; the exactly-one-builder assertion remains unchanged.
- CLI cascade assertions inspect the typed-classifier cascade and the escalation
  macro, not earlier terminal-owner or pre-prompt call sites. The network marker
  follows its current shared recovery helper.
- NAPI fixtures isolate credentials, configuration, and model cache; restore
  environment on drop; serialize global mutation; and check explicit session
  creation before invoking either real compaction producer. Deferred-loop hooks
  retain the real input receiver so an acknowledgement can queue continuation
  without starting inference or dropping its channel.

Every Cargo command below uses `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`, runs from `rust/`, and scopes package, target, and
parallelism (`-j 2`). Logs retain compiler output and behavioral assertions.

| Phase | Command targets | Log and result |
| --- | --- | --- |
| Red | `codelet-cli`: `cmpct044_cascade_ordering_test`, `cmpct057_operation_lifecycle`, `compaction_cancel_preservation_test`, `compaction_retry_prompt_policy_test` | `/tmp/BUG-210-legacy-cli-red.log`: ordering, durable-source and provider-selection failures |
| Red | `codelet-agent-loop`: `cmpct044_terminal_overflow_test` | `/tmp/BUG-210-legacy-agent-red.log`: boolean assertions do not handle the fallible recovery API |
| Red | `codelet-agent-loop`: `cmpct045_generate_compaction_behavioral` | `/tmp/BUG-210-cmpct045-red.log`: 8 passed, 17 failed |
| Red | `codelet-napi --features __full_runtime`: `cmpct038_measurement_basis_test`, `cmpct039_ratio_clamp_test`, `rpc421_honest_ack_test`, `session_bindings_shape` | `/tmp/BUG-210-legacy-napi-red.log`: invalid session IDs and retired apply-path expectations; architecture target interrupted after spawning nested broad Cargo checks |
| Green | `codelet-cli`: cancellation, retry, CMPCT-057 targets above | `/tmp/BUG-210-cli-fixtures-final.log`: 14 passed; the cascade target still failed in this intermediate run |
| Green | `codelet-cli`: `cmpct044_cascade_ordering_test` | `/tmp/BUG-210-cascade-green.log`: 5 passed after final source-window repair |
| Green | `codelet-agent-loop`: `bug210_automatic_continuation`, `bug210_compaction_lifecycle`, `bug210_completion_metrics` | `/tmp/BUG-210-core-revalidation.log`: 15 passed |
| Green | `codelet-fspec-tui`: `automatic_compaction_progress_bug210` | `/tmp/BUG-210-tui-revalidation.log`: 3 passed |
| Blocked | `codelet-napi --features __full_runtime`: `bug210_shared_compaction` | `/tmp/BUG-210-napi-revalidation.log`: shared RPC-type compilation failure |
| Blocked | `codelet-napi --features __full_runtime`: ratio and acknowledgement targets | `/tmp/BUG-210-napi-fixtures-green.log`: same compilation failure, before these tests execute |
| Green | `codelet-napi --features __full_runtime`: `bug210_shared_compaction` | `/tmp/BUG-210-napi-fixtures-final.log`: 1 passed after the shared blocker was resolved; the two legacy targets still exposed closed input channels in this intermediate run |
| Green | `codelet-napi --features __full_runtime`: `cmpct039_ratio_clamp_test`, `rpc421_honest_ack_test` | `/tmp/BUG-210-napi-fixtures-green-final.log`: 8 passed after retaining the continuation receiver |

The temporary shared blocker was `rpc-types/src/lib.rs`: the new
`StreamChunk::Interrupted.reason` uses `InterruptionReason`, which lacks the
`ToNapiValue` implementation required when the NAPI feature is enabled (E0277).
Its production owner resolved it without production edits from this follow-up.
All 19 BUG-210 scenarios are now freshly green. The subsequent legacy fixture
run and its scoped quality checks are recorded separately from those scenario
results; the broader legacy gates below still prevent completion.

Remaining legacy-contract migration is not a fixture-only change:
`cmpct045_generate_compaction_behavioral` still expects fabricated fallback
success and raw tool-result text, whereas current operation-owned compaction
fails closed and self-target results acknowledge queuing. CMPCT-038 still invokes
the retired raw pending-DAG apply path. Do not restore those paths or weaken
assertions to manufacture a green gate. CMPCT-044's terminal tests still require
fallible-API and durable-source migration. The wrapper-count baseline conflict
also remains unresolved. BUG-210 is not ready to move to `done`.

### Follow-up quality evidence

- CLI scoped check and Clippy pass:
  `/tmp/BUG-210-cli-fixtures-check.log` and
  `/tmp/BUG-210-cli-fixtures-clippy.log`.
- NAPI scoped check and Clippy pass after the final fixture repair:
  `/tmp/BUG-210-napi-fixtures-check-final.log` and
  `/tmp/BUG-210-napi-fixtures-clippy-final.log`. Seven existing production warnings
  remain; this is not a warning-free build claim.
- Changed-file rustfmt and diff whitespace checks pass:
  `/tmp/BUG-210-fixtures-fmt-final.log` and
  `/tmp/BUG-210-diff-check-final.log`.
- Eleven feature coverage audits pass, including the five BUG-210 features and
  six repaired legacy targets. All report 100% scenario mappings. Updated test
  ranges preserve existing implementation links and mandatory step validation;
  no duplicate mappings remain. Logs:
  `/tmp/BUG-210-legacy-coverage-final.log` and
  `/tmp/BUG-210-coverage-final-audit.log`.
- Gherkin validation passes for 2,034 features:
  `/tmp/BUG-210-features-revalidation.log`. Repository tag validation remains red
  (1,505 passing files, 529 files with violations):
  `/tmp/BUG-210-tags-revalidation.log`.
- Board status remains `validating`. No staging, commit, branch change, direct
  board JSON edit, or production edit belongs to this follow-up. Every mutating
  fspec command used the shared `/tmp/fspec-acdd-board.lock` mutex.

Exact final behavioral invocations (with both debug-profile environment variables
set to zero as described above):

```text
cargo test -p codelet-agent-loop --test bug210_automatic_continuation --test bug210_compaction_lifecycle --test bug210_completion_metrics -j 2 --no-fail-fast
cargo test -p codelet-fspec-tui --test automatic_compaction_progress_bug210 -j 2
cargo test -p codelet-napi --features __full_runtime --test cmpct039_ratio_clamp_test --test rpc421_honest_ack_test --test bug210_shared_compaction -j 2 --no-fail-fast
cargo test -p codelet-cli --test compaction_cancel_preservation_test --test compaction_retry_prompt_policy_test --test cmpct057_operation_lifecycle --test cmpct044_cascade_ordering_test -j 2 --no-fail-fast
cargo test -p codelet-cli --test cmpct044_cascade_ordering_test -j 2
cargo test -p codelet-napi --features __full_runtime --test cmpct039_ratio_clamp_test --test rpc421_honest_ack_test -j 2 --no-fail-fast
```

The combined CLI and combined NAPI invocations include intermediate failures;
their final focused reruns establish the green results. Overall fresh evidence:
19 BUG-210 tests and 27 repaired legacy tests pass; no package-wide green claim.

### Exact follow-up file list

- `rust/cli/tests/cmpct044_cascade_ordering_test.rs`
- `rust/cli/tests/cmpct057_operation_lifecycle.rs`
- `rust/cli/tests/compaction_cancel_preservation_test.rs`
- `rust/cli/tests/compaction_retry_prompt_policy_test.rs`
- `rust/napi/tests/cmpct039_ratio_clamp_test.rs`
- `rust/napi/tests/rpc421_honest_ack_test.rs`
- `rust/napi/tests/support/legacy_compaction_fixture.rs` (new, 89 lines)
- `spec/features/committed-compaction-watchdog-lifecycle.feature.coverage`
- `spec/features/compactor-sub-agent-escalation-in-stream-loop-error-cascade.feature.coverage`
- `spec/features/preserve-partial-assistant-text-and-token-tracker-on-hook-triggered-compaction.feature.coverage`
- `spec/features/correct-post-compaction-retry-prompt-semantics.feature.coverage`
- `spec/features/compression-ratio-clamping.feature.coverage`
- `spec/features/honest-compaction-acknowledgement.feature.coverage`
- `spec/attachments/BUG-210/BUG-210-validation.md`
