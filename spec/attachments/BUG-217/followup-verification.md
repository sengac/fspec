# BUG-217 independent delivery and prefix reclamation follow-up

This follow-up addresses both review findings against commit `3bd3e3a4`.
Nothing is staged or committed; the parent owns the separate follow-up commit.

## Behavior and resource bounds

- A forwarding owner maintains one independent journal cursor and delivery task
  per connected relay, with one in-flight event and the existing 256-slot channel.
  It never creates a task or an in-memory backlog per emitted chunk.
- A stalled relay cannot block a healthy relay. Resuming the stalled relay replays
  all of its events exactly once in order, including completion, with root routing.
- Dropping a relay receiver releases its worker even without another source event.
  Aborting the forwarding owner aborts its `JoinSet` workers and releases cursors.
- Late relays receive newly forwarded output, not already-forwarded history.
  Source destruction drains pending completion to live relays before owner exit.
- Before publication, a common consumed prefix is compacted when it is at least
  64 KiB and at least as large as the remaining unread suffix. All-caught-up
  journals still truncate immediately before publication. Copying uses bounded
  `std::io::copy` memory, preserves absolute cursors, and retains every unread event.
  Between compactions, retained bytes are bounded by roughly twice unread bytes
  plus 64 KiB and the next event, rather than total historical output.

## ACDD and behavioral red

Reopened BUG-217 under the cross-agent fspec mutex, captured two rules and three
examples, wrote two feature files, then five corresponding integration tests
with exact `@step` comments before the production changes.

Every Cargo command below ran from `rust/` with
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`.

```sh
cargo test -p codelet-agent-loop --features test-support --test bug217_independent_relays -j 2
cargo test -p codelet-sessions --test bug217_journal_reclamation -j 2
```

- `/tmp/bug217-followup-forwarding-red.log`: healthy-relay delivery times out
  while the first relay remains undrained; 1 failed, 2 passed. The final red run
  uses the production JSON converter. Earlier fixture compile/conversion errors
  were corrected and are not counted as behavioral evidence.
- `/tmp/bug217-followup-journal-red.log`: both storage regressions fail. Continuous
  three-record lag retains 4,248,890 bytes; cancelling the slowest cursor still
  leaves 426,814 bytes. Both regressions require less than 128 KiB.

## Green and quality gates

```sh
cargo test -p codelet-agent-loop --features test-support --test bug217_independent_relays --test bug217_subordinate_forwarding --test subordinate_manifest_single_writer -j 2 --no-fail-fast
cargo test -p codelet-sessions --test bug217_journal_reclamation -j 2
cargo check -p codelet-sessions -p codelet-agent-loop --features codelet-agent-loop/test-support --lib -j 2
cargo clippy -p codelet-sessions --lib --test bug217_journal_reclamation -j 2 -- -D warnings
cargo clippy -p codelet-agent-loop --features test-support --lib --test bug217_independent_relays --test bug217_subordinate_forwarding -j 2 -- -D warnings
```

- **12 passed, zero failed**: 3 independent-relay tests, 2 reclamation tests,
  4 original forwarding regressions, and 3 subordinate-manifest regressions.
- Logs: `/tmp/bug217-followup-forwarding-green.log`,
  `/tmp/bug217-followup-journal-green.log`, `/tmp/bug217-followup-check.log`,
  `/tmp/bug217-followup-sessions-clippy.log`, and
  `/tmp/bug217-followup-agent-loop-clippy.log`.
- Scoped `rustfmt --edition 2021 --check` passes for both production files and
  both new integration targets: `/tmp/bug217-followup-fmt.log`.
- Syntax, registered tags, formatting and coverage audits pass for all three
  BUG-217 feature files. All 5 new scenarios map one-to-one to tests and
  implementations; the 4 historical scenario mappings remain covered and their
  implementation locations are refreshed. `fspec review BUG-217` passes.
- Final specification/coverage/review and whitespace evidence is recorded in
  `/tmp/bug217-followup-spec-quality.log`.

The journal storage test includes the actual journal source inside its test module
to inspect real tempfile size without adding a production-only testing API. The
forwarding tests exercise the real shared session/front-door path and real channels.

## Exact follow-up file list

1. `rust/sessions/src/subordinate_forwarding.rs`
2. `rust/sessions/src/subordinate_output.rs`
3. `rust/agent-loop/tests/bug217_independent_relays.rs`
4. `rust/sessions/tests/bug217_journal_reclamation.rs`
5. `spec/features/independent-subordinate-relay-progress.feature`
6. `spec/features/independent-subordinate-relay-progress.feature.coverage`
7. `spec/features/subordinate-journal-prefix-reclamation.feature`
8. `spec/features/subordinate-journal-prefix-reclamation.feature.coverage`
9. `spec/features/subordinate-forwarding-lag-recovery.feature`
10. `spec/features/subordinate-forwarding-lag-recovery.feature.coverage`
11. `spec/attachments/BUG-217/followup-verification.md`
12. `spec/work-units.json` (BUG-217 metadata only; shared concurrent changes exist)

Both production files and every new file remain below 300 lines. No bridge API,
front-door signature, session interruption fields, other cards, or tag registry
were changed. Existing bounded bridge channels are reused unchanged.

## Remaining limitations

Journal I/O and compaction remain synchronous and transient, not crash recovery.
A genuinely stalled, still-connected relay necessarily retains unread output on
disk until it resumes, disconnects, or its forwarding owner is cancelled; other
relays are independent. Reclamation occurs before subsequent publication, not as
an idle background vacuum. No live WebSocket/load or filesystem-fault test was run.
Cargo gates queued on shared artifact/build locks; no broad tests were run.
