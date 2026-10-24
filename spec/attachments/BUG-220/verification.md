# BUG-220 verification

## Behavior

Session control retains typed `user` or `loopDetector` provenance separately
from the pending corrective note. User interruption wins in either arrival
order, discards the note, and advances the cancellation epoch. A synthetic
loop retry can clear only its own detector interruption at the captured epoch;
a later user stop remains authoritative, including after retry admission.

CLI teardown reads that cause when recording unfinished committed tool calls.
Completed results, invocation identities, accepted-completion handling, and
cancellation-before-Interrupted/Done ordering are preserved. BackgroundOutput
publishes the typed cause and persists `interruptionReason` only on interruption,
not ordinary completion. Retry logs identify loop recovery versus compaction.

The unchanged legacy `Interrupted` wire variant retains its bincode layout and
index. `InterruptedWithReason` is appended, with camelCase reason strings across
Serde, bridge JSON and the optional NAPI binding. Manual bridge events retain
`type: interrupted`. The TUI distinguishes explicit user and detector causes;
legacy reasonless events retain the generic notice.

## Acceptance coverage

- `typed-interruption-provenance`: 3 scenarios / 3 tests.
- `loop-interruption-retry-admission`: 2 scenarios / 2 tests.
- `interruption-wire-compatibility`: 2 scenarios / 2 tests.
- `interruption-reason-rendering`: 1 scenario / 1 test.

Each feature links exactly one test file. Each scenario links exactly one test,
with matching `@step` comments and production implementation anchors.

## Commands and evidence

All Cargo commands run from `rust/` with
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` and `-j 2`.
No workspace-wide or unscoped tests were run.

### Red

1. `cargo test -p codelet-agent-loop --test bug220_interruption_provenance -j 2`
   — `/tmp/bug220-provenance-red.log`: missing detector reason and user-stop
   precedence reproduced. The third failure initially concerned fixture lifetime,
   not a proven missing-cancellation-record defect; see limitations below.
2. `cargo test -p codelet-agent-loop --test bug220_loop_retry --features test-support -j 2`
   — `/tmp/bug220-retry-red.log`: both tests fail on missing typed reasons or
   retained recovery after user stop.
3. `cargo test -p codelet-rpc-server --test bug220_interruption_wire -j 2`
   — `/tmp/bug220-wire-red.log`: both tests reject the absent typed variant.
4. `cargo test -p codelet-fspec-tui --test bug220_interruption_rendering -j 2`
   — `/tmp/bug220-render-red.log`: typed variant absent. An earlier attempt was
   blocked by an unrelated concurrent compaction-runway import error.

### Green

The same four acceptance commands pass: 3 + 2 + 2 + 1 = 8 tests.
Logs: `/tmp/bug220-provenance-green.log`, `/tmp/bug220-retry-green.log`,
`/tmp/bug220-wire-green.log`, `/tmp/bug220-render-green.log`.

Additional focused regression commands:

```text
cargo test -p codelet-cli --test codex_interruption_outcomes --test codex_accepted_completion_batch --test codex_interruption_call_pairs --test codex_restored_interruption -j 2
cargo test -p codelet-agent-loop --test rig015_loop_abort_behavioral --features test-support -j 2
cargo test -p codelet-fspec-tui --test chunk_rendering_parity_rpc071 --test chunk_rendering_parity_rpc078 --test per_message_table_finalization_rpc432 -j 2
```

These pass 5, 3 and 25 tests respectively. Logs:
`/tmp/bug220-cli-regressions.log`, `/tmp/bug220-detector-regressions.log`,
`/tmp/bug220-tui-regressions.log`.

### Quality gates

```text
cargo check -p codelet-agent-loop -p codelet-rpc-server -p codelet-fspec-tui --features codelet-agent-loop/test-support -j 2
cargo clippy -p codelet-agent-loop --features test-support --test bug220_interruption_provenance --test bug220_loop_retry -j 2
cargo clippy -p codelet-rpc-server --test bug220_interruption_wire -j 2
cargo clippy -p codelet-fspec-tui --test bug220_interruption_rendering -j 2
cargo check -p codelet-napi --features __full_runtime -j 2
cargo clippy -p codelet-rpc-types --features napi -j 2
cargo clippy -p codelet-cli --test codex_interruption_outcomes --test codex_accepted_completion_batch -j 2
cargo clippy -p codelet-sessions --lib -j 2
cargo clippy -p codelet-napi --features __full_runtime --lib -j 2
```

All checks and Clippy commands pass, including the optional NAPI surface.
Logs: `/tmp/bug220-check.log`,
`/tmp/bug220-clippy-agent.log`, `/tmp/bug220-clippy-wire.log`,
`/tmp/bug220-clippy-tui.log`, `/tmp/bug220-napi-check.log`,
`/tmp/bug220-clippy-napi-types.log`, `/tmp/bug220-clippy-cli.log`,
`/tmp/bug220-clippy-sessions.log`, `/tmp/bug220-clippy-napi.log`.

Scoped `rustfmt --edition 2021 --check --config skip_children=true` covers every
owned Rust source/test file; `/tmp/bug220-fmt.log` is empty on success.
`git diff --check` passes. Each owned feature passes `fspec validate` and
`fspec audit-coverage` (`/tmp/bug220-spec-gates.log`); `fspec review BUG-220` reports PASS
(`/tmp/bug220-review.log`). Repository tag validation still reports 529 unrelated
features; none of the four owned features appears in those failures
(`/tmp/bug220-tags.log`).

All mutating fspec commands run through the cross-agent mutex:

```text
python3 -c 'import fcntl,subprocess,sys; handle=open("/tmp/fspec-acdd-board.lock","w"); fcntl.flock(handle,fcntl.LOCK_EX);sys.exit(subprocess.call(sys.argv[1:]))' fspec ...
```

## Owned file list

Production:

- `rust/rpc-types/src/lib.rs`
- `rust/sessions/src/background_session.rs`
- `rust/sessions/src/background_session/interruption.rs` (new)
- `rust/cli/src/interactive/output.rs`
- `rust/cli/src/interactive/interruption.rs`
- `rust/cli/src/interactive/stream_loop.rs` (interruption branch only)
- `rust/agent-loop/src/agent_loop.rs` (loop retry admission and labels only)
- `rust/agent-loop/src/background_output.rs` (cause propagation/persistence only)
- `rust/agent-loop/src/persist.rs`
- `rust/agent-loop/src/stream_chunk_json.rs`
- `rust/agent-loop/src/terminal_overflow_recovery.rs` (retry epoch payload only)
- `rust/napi/src/types.rs`
- `rust/fspec-tui/src/store/agent_view/record_chunk.rs`

Tests/build metadata:

- `rust/agent-loop/tests/bug220_interruption_provenance.rs` (new)
- `rust/agent-loop/tests/bug220_loop_retry.rs` (new)
- `rust/agent-loop/tests/support/bug220.rs` (new)
- `rust/rpc-server/tests/bug220_interruption_wire.rs` (new)
- `rust/fspec-tui/tests/bug220_interruption_rendering.rs` (new)
- `rust/rpc-server/Cargo.toml` (test-helper dev dependency only)
- `rust/Cargo.lock` (corresponding dependency edge only)

Specifications/workflow:

- `spec/features/typed-interruption-provenance.feature` (new)
- `spec/features/typed-interruption-provenance.feature.coverage` (new)
- `spec/features/loop-interruption-retry-admission.feature` (new)
- `spec/features/loop-interruption-retry-admission.feature.coverage` (new)
- `spec/features/interruption-wire-compatibility.feature` (new)
- `spec/features/interruption-wire-compatibility.feature.coverage` (new)
- `spec/features/interruption-reason-rendering.feature` (new)
- `spec/features/interruption-reason-rendering.feature.coverage` (new)
- `spec/attachments/BUG-220/verification.md` (new)
- `spec/work-units.json` (BUG-220 workflow only, via mutex)

Existing research remains untouched. Other workers own unrelated hunks/files in
the shared worktree. No staging, commits, branches, or live provider requests.

## Limitations and remaining risks

- Tests use real detector/session/stream teardown, local HTTP provider fixtures,
  real persistence, and embedded/WebSocket transport. They call the same session
  stop primitive as Escape; they do not synthesize physical keyboard input.
- The existing Rig tool-server actor owns already-dispatched tool futures beyond
  stream teardown. The running-tool fixture therefore releases its blocked tool
  after asserting the published cancellation contract. This change preserves
  exactly-once recorded outcomes; it does not claim to fix that existing actor
  lifetime or make arbitrary tool side effects reversible.
- Compatibility covers new readers accepting old reasonless events and unchanged
  legacy binary layouts. An old binary decoder cannot understand a newly added
  typed enum variant without upgrading; no protocol-version negotiation is added.
- Existing fixture dead-code warnings and unrelated TUI/NAPI warnings remain.
- Initial check/regression attempts encountered transient concurrent edits in
  subordinate-output files. A later NAPI Clippy pass briefly encountered an
  undefined `model_string` during a concurrent legacy-model edit. Those unrelated
  files were not modified for BUG-220; the final scoped reruns pass.
- All quality gates pass. BUG-220 is done and all four owned features are tagged
  `@done`. The changes remain uncommitted for the supervisor's final review.
