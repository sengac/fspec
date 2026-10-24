# BUG-219 verification

## Change

`RecoveryGuard::drop` publishes `Cancelled` only when the unfinished stream's
session handle is actively waiting for network or retrying a stream. Ordinary
initial setup, body waiting and healthy partial-stream drops leave recovery
status unset and emit no recovery notification. Resource ownership and the
existing interruption protocol are unchanged.

The existing offline cancellation regression now distinguishes ordinary initial
setup/body waiting from genuine offline recovery instead of requiring a false
cancellation notification for all three cases.

## ACDD evidence

Example Mapping: two rules and three examples on BUG-219, informed by
`research.md`. Specifications preceded new tests; both behavioral regressions
failed before the production change. The session-output scenario was split
into its own feature to satisfy fspec's one-feature-to-one-test-file rule.

All Cargo commands run from `rust/` with
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` and `-j 2`.

### Red

```sh
cargo test -p codelet-providers --test bug219_codex_interruption_status -j 2
cargo test -p codelet-agent-loop --test bug219_codex_interruption_output -j 2
```

- `/tmp/bug219-provider-red.log`: 1 failed, 2 passed. Ordinary cancellation
  produced `Some(Cancelled)` instead of `None`.
- `/tmp/bug219-output-red.log`: 1 failed. A healthy partial-stream drop through
  the real session provider observer produced `Some(Cancelled)` instead of
  `None`.

### Green

```sh
cargo test -p codelet-providers --test bug219_codex_interruption_status -j 2
cargo test -p codelet-providers --test prov160_codex_offline -j 2
cargo test -p codelet-providers --test prov160_codex_reconnection -j 2
cargo test -p codelet-providers --test prov160_reconnect_stream_consistency -j 2
cargo test -p codelet-agent-loop --test bug219_codex_interruption_output -j 2
cargo test -p codelet-agent-loop --test prov160_reconnect_status -j 2
```

Results: 3 + 2 + 5 + 3 + 1 + 1 = 15 passing tests, zero failures.
Logs respectively:

- `/tmp/bug219-provider-green.log`
- `/tmp/bug219-offline-green.log`
- `/tmp/bug219-reconnection-green.log`
- `/tmp/bug219-consistency-green.log`
- `/tmp/bug219-output-green.log`
- `/tmp/bug219-status-green.log`

New tests cover silent setup/body cancellation with actual socket closure;
offline and stream-retry cancellation exactly once with no later request;
healthy and recovered final/tool completion, including handle reuse; and the
session broadcast output consumed by the TUI. All transport fixtures bind
loopback; no live provider requests are made.

## Quality commands

```sh
cargo check -p codelet-providers -p codelet-agent-loop -j 2
cargo clippy -p codelet-providers --lib --test bug219_codex_interruption_status --test prov160_codex_offline -j 2
cargo clippy -p codelet-agent-loop --lib --test bug219_codex_interruption_output --test prov160_reconnect_status -j 2
```

Logs: `/tmp/bug219-check.log`, `/tmp/bug219-providers-clippy.log`,
`/tmp/bug219-output-clippy.log`.
All three commands finish successfully; Clippy reports only inherited shared
fixture dead-code and module-inception warnings.

From the repository root:

```sh
rustfmt --edition 2021 --check rust/providers/src/codex/reconnect.rs rust/providers/tests/bug219_codex_interruption_status.rs rust/providers/tests/prov160_codex_offline.rs rust/agent-loop/tests/bug219_codex_interruption_output.rs
fspec validate spec/features/codex-interruption-status.feature
fspec validate spec/features/codex-interruption-output.feature
fspec audit-coverage codex-interruption-status
fspec audit-coverage codex-interruption-output
fspec review BUG-219
```

Formatting log: `/tmp/bug219-fmt.log`. Both coverage audits pass. Coverage links
map each of the four scenarios to exactly one test and the relevant existing
implementation, including the session observer for the integration scenario.
The features report 3/3 and 1/1 scenarios fully covered. `fspec review` passes
without warnings (`/tmp/bug219-review.log`).

Scoped `validate-tags` through the fspec dispatcher with
`{"file":"spec/features/codex-interruption-status.feature"}` and
`{"file":"spec/features/codex-interruption-output.feature"}` returns
`validCount: 1, invalidCount: 0` for each feature. The repository-wide CLI
`fspec validate-tags` reports 529 unrelated feature violations and 1488 passing
files (`/tmp/bug219-tags.log`); no unrelated tag registry changes are made.

## Scope and limitations

The session-output test exercises the shared stream-drop boundary used by user
and internal interruption, not synthetic Escape keystrokes or loop-detector
triggering. Neither caller's Interrupted/Done behavior changes. This fix does
not change loop detection, interruption reasons, or compaction admission.
Inherited shared fixture dead-code and module-inception warnings are not
suppressed. No Git staging, commits, or branch changes are performed.
