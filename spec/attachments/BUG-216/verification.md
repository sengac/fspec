# BUG-216 verification and commit handoff

Status: done. No staging or commit performed.

## Changes

- Streaming tokenization retains unfinished words across deltas and bounds carry storage to 4096 bytes. Oversized words break the evidence window rather than manufacturing fragments.
- Long suffix detection counts configured non-overlapping earlier copies, excluding the trailing copy. The default three requires a fourth copy.
- BackgroundOutput resets the warned channel's detector, requiring independent new evidence before escalation. Public latched detector diagnostics remain compatible.
- No interruption protocol or provenance changes were made.
- Existing detector fixtures now contain actual whitespace between streamed words; the profile fixture supplies sufficient sustained repetition for independent warning and abort evidence.

## Exact file list

```text
rust/agent-loop/src/background_output.rs
rust/agent-loop/src/stream_loop_detector.rs
rust/agent-loop/tests/bug216_fresh_loop_escalation.rs
rust/agent-loop/tests/bug216_loop_abort_retry.rs
rust/agent-loop/tests/bug216_stream_word_evidence.rs
rust/agent-loop/tests/bug216_stream_word_evidence.proptest-regressions
rust/agent-loop/tests/prov145_profile_config_applied.rs
rust/agent-loop/tests/rig014_streaming_loop_detection.rs
rust/agent-loop/tests/rig015_loop_abort_behavioral.rs
spec/features/streaming-loop-abort-retry-regression.feature
spec/features/streaming-loop-abort-retry-regression.feature.coverage
spec/features/streaming-loop-false-positive-regressions.feature
spec/features/streaming-loop-false-positive-regressions.feature.coverage
spec/features/streaming-loop-fresh-escalation.feature
spec/features/streaming-loop-fresh-escalation.feature.coverage
spec/features/streaming-loop-detection.feature
spec/attachments/BUG-216/verification.md
spec/work-units.json
```

Only BUG-216 metadata was mutated in the shared work-unit file. The research attachment existed before this assignment and was read, not edited. Other workers' changes must not be attributed to this patch.

## ACDD and coverage

Example Mapping: three rules, three examples, one architecture note. Features preceded tests; behavioral failures preceded production edits. Status followed backlog, specifying, testing, implementing, validating, done. All mutating fspec commands used the cross-agent mutex.

Eight scenarios map one-to-one to tests with exact step comments across three features: five word-evidence scenarios, two escalation scenarios, one full-loop retry scenario. Each feature has one integration-test target. All scenarios have implementation mappings; all three coverage audits passed. New feature syntax and registered tags passed; their final status tags are done. Fspec review reported PASS.

## Commands and evidence

Run Cargo commands from `rust/`. Every invocation used both zero-debug environment settings and two jobs.

Initial red command:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p codelet-agent-loop --features test-support --test bug216_stream_word_evidence --test bug216_fresh_loop_escalation --test bug216_loop_abort_retry -j 2 --no-fail-fast
```

Behavioral red evidence is in `/tmp/bug216-red.log`: chunk-invariance, configured threshold, overlapping evidence, and partial-word failures. The integration test demonstrated premature aborts across eleven attempts. After correcting a session-fixture setup error, the focused escalation test failed on a fresh delta interrupting from latched evidence; see `/tmp/bug216-escalation-red.log`.

Final green command:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p codelet-agent-loop --features test-support --test bug216_stream_word_evidence --test bug216_fresh_loop_escalation --test bug216_loop_abort_retry --test rig014_streaming_loop_detection --test rig015_loop_abort_behavioral --test prov145_profile_config_applied -j 2 --no-fail-fast
```

Result: 35 passed, zero failed. Log: `/tmp/bug216-green.log`. This includes proptest chunk partitioning and threshold variation, channel isolation, reset behavior, actual provider-stream cancellation, persistence, corrective retry context, and termination after the default ten retries.

Compilation and lint:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo check -p codelet-agent-loop --features test-support --lib -j 2
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy -p codelet-agent-loop --features test-support --lib --test bug216_stream_word_evidence --test bug216_fresh_loop_escalation --test bug216_loop_abort_retry -j 2 -- -D warnings
```

Both completed successfully. Logs: `/tmp/bug216-check.log`, `/tmp/bug216-clippy.log`.

Scoped formatting check:

```sh
rustfmt --check --edition 2021 --config skip_children=true rust/agent-loop/src/stream_loop_detector.rs rust/agent-loop/src/background_output.rs rust/agent-loop/tests/bug216_stream_word_evidence.rs rust/agent-loop/tests/bug216_fresh_loop_escalation.rs rust/agent-loop/tests/bug216_loop_abort_retry.rs rust/agent-loop/tests/rig014_streaming_loop_detection.rs rust/agent-loop/tests/rig015_loop_abort_behavioral.rs rust/agent-loop/tests/prov145_profile_config_applied.rs
```

Passed; `/tmp/bug216-fmt.log` is empty. Scoped git whitespace checking passed.

## Remaining limitations and unrelated findings

- Live Codex notification sequencing was not exercised. The regression uses the real production agent loop with the existing deterministic stub provider. BUG-219 and BUG-220 remain outside this assignment.
- The historical streaming-loop-detection feature already has tag-validation violations: scenario-level RIG-014 tags and missing current required feature-level categories. Its syntax passes; only its contradictory prose example was corrected. The three new BUG-216 features pass tag validation.
- A whole-tree whitespace check encountered an unrelated trailing blank line in `rust/sessions/tests/cont009_completion_contract_sync.rs`; it was not changed. Scoped checks for this patch pass.
- Existing production files already exceed the repository's preferred size limit. No major refactor was attempted; all new files are below 300 lines.
