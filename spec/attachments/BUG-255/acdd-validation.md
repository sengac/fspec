# BUG-255: ACDD execution and validation

## Authorization and native lifecycle

The original RLCD hold rejected the implementing transition before execution.
Native history still contained only specifying/testing when this task resumed.
The human was informed that this was a workflow hold, not failing prerequisites
or a pending AskUser approval, and said at 06:57 UTC on 2026-10-25:

> no just approve it, get 255 done

One normal native implementing retry included that materially new informed
authorization and exact fresh red-test evidence in `reason`. It succeeded at
**07:03:53.307 UTC**. No second wording/session, AllowSession, fabricated pending
approval, gate configuration change or ungated path was used. Production repair
started only after success. See `authorization-and-transition-retry.md`.

The discovery question was answered, with no blocking dependencies/children.
The four existing scenarios and test mappings preceded production repair.
Formatting moved test line ranges; native link initially appended current ranges
beside old ranges. The validating coverage gate correctly rejected the stale
unimplemented mappings. Native unlink/link commands removed stale/duplicate
entries, leaving exactly four scenarios with one current test mapping and
nonempty implementation mapping each. No JSON file was edited manually.
Native transition to validating then succeeded at **07:15:46.099 UTC**.

## Preserved red evidence

From `rust/`, before any repair or retry:

```text
cargo test --offline --profile ci-test -p codelet-core --test bug255_append_scaling -j2
```

`red-current.log`: **2 passed, 2 failed**, exit **101**, no ignored/filtered
acceptance tests. The same original behaviors reproduce:

- 256 deltas reread **33,664** historical records.
- The unrelated session is blocked behind the global recovery mutex while the
  first writer is held at the real filesystem lock.

Cold intact/corrupt/incomplete reconstruction and exact durable retry identity
pass. The original historical record and its 14 passing related controls remain
in `acdd-red-and-native-hold.md`, not silently rewritten into a successful retry.

Before production changes, the cold scenario gained warm tail-corruption,
missing-interior and second-writer invalidation controls. The scaling scenario
gained actual recovery-path scaling, not just direct store calls.
`red-extended.log` again completed with the same **2 red/2 green** result.
The first two implementation compilation attempts exposed a leftover return and
incomplete mechanical owner substitutions; those were corrected without changing
acceptance expectations. Their logs are not counted as behavioral red evidence.

## Connected repair

- `transcript/admission.rs` owns incremental identity digests, request/sequence
  coordinates, lifecycle states and directory/tail freshness stamps. It does not
  retain old text or tool-result payloads in the warm admission index.
- `transcript/lifecycle.rs` shares the same tool/anchor transition validation
  between cold projection and incremental admission; `projection.rs` delegates
  validation without changing its safe display projection.
- `transcript/writer.rs` cold-validates history under the existing writer lease,
  then validates only new deltas while stamps match. Directory/tail changes cause
  reconstruction. Identity retries verify exact durable bytes. Cache installation
  follows existing record/directory/root acknowledgement; errors discard it.
- `JournalStore::read` still validates every record and invalidates warm state.
  Configured appends keep the existing committed-session ownership transaction,
  but reuse the same writer instead of creating a cold store inside the closure.
- Recovery's global registry only clones session-lane handles or prunes idle
  entries without filesystem work. A per-session mutex owns recovery originals
  and the retained journal writer across successful retries. Deleted owners retire
  their lane and clear memory; unreadable manifests preserve pending originals.
- Idle cache pruning is exercised under pressure from 129 deleted-owner opens.
  An active writer remains warm, and all deleted-owner admissions are rejected.

This is the already specified incremental-writer/per-session-lane repair, not a
new runtime engine, storage format, publication hook or broad redesign. All
touched/new Rust files are below 300 lines; the largest touched test is 258.
No BUG-258 terminal routing or other completed-card implementation is edited.

## Final executed test ledger

All tests run from `rust/`, offline, with `--profile ci-test`, explicit `-p`,
explicit `--test` targets and `-j2`. Multi-target runs use `--no-fail-fast`.
No workspace/unscoped tests, downloads or cache deletions were used.

| Package and full target scope | Parent test passes | Evidence |
| --- | --- | --- |
| Core: `bug249_journal_admission`, `bug249_journal_integrity`, `bug249_transcript_journal`, `bug250_transcript_lifecycle` | 14 | `regression-integrity.log` |
| Agent-loop: `bug246_persistence_recovery`, `bug246_recovery_admission`, `bug246_journal_promotion`, `bug246_recovery_boundaries`, `bug246_session_ownership`, `bug246_corrupt_admission`, `bug250_terminal_archive`, `bug250_request_anchors`, `bug250_deleted_recovery`, `bug250_recovery_retirement` | 26 | `regression-recovery.log` |
| Agent-loop: `bug249_admission_ownership`, `bug249_executor_gate`, `bug249_interrupted_tool_state`, `bug249_stream_crash`, `bug253_gemini_invocations`, `bug257_fork_continuation`, `bug258_outer_terminal`, `prov149_codex_reload` | 37 | `regression-streaming.log` |
| Core: `bug255_append_scaling`, `bug250_transcript_outcomes`, `bug250_request_boundaries`, `bug250_archival_journal` | 13 | `final-green.log` |
| Sessions: `bug245_cold_empty_failures`, `bug250_transcript_replay` | 2 | `regression-cold.log` |
| Providers: `prov149_codex_history` | 2 | `regression-provider-history.log` |
| Agent-loop: `bug210_automatic_continuation`, `cmpct053_terminal_lock`, `cmpct054_cancellation_cleanup`, `cmpct054_bridge_clear_failure`, `rpc089_mcp_injection_drain` | 18 | `regression-cancellation-mcp.log` |

**112 distinct scoped parent tests passed, zero failures in this executed final
matrix.** All commands exited **0**. Child-process helper entries remain marked
ignored in their parent harnesses, but are explicitly invoked by parent tests;
their repetitions are not counted again. No BUG-255 acceptance test is ignored.

The initial repaired `green.log` has all four acceptance cases passing. The
definitive `final-green.log` repeats all four after adding active-lane eviction
and deleted-owner pressure checks. Growing batches are 256/512/1024 deltas,
after authentic completed tool history. Tests bound historical reads and
lifecycle admissions per batch. Another 256 actual recovery-path records, plus
one after registry pressure, verify the retained writer is connected and warm.

## Quality and native specification checks

Executed successfully from `rust/`:

```text
cargo check --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-core --test bug255_append_scaling -j2
```

`check.log`, `clippy.log`, `clippy-test.log`: exit **0**, no warnings in these
outputs. Some existing shared fixtures warn about unused members in regression
test compilation; no lint is disabled and no unrelated fixture is changed.

Native scoped feature syntax and registered-tag validation pass. Native
`audit-coverage` finds all **8/8** mapped file references valid; direct inspection
confirms four 1:1 mappings, current line ranges and nonempty implementation
links. Targeted rustfmt check and `git diff --check` pass. Direct implementation
review is recorded in `review-findings.md`, performed in this same session.

## Limits and preserved state

This is not a whole-workspace all-green claim. The five broader failures already
classified on BUG-258 and existing global specification tag/format warnings are
not repaired or rerun here. BUG-258 preserves their executed baseline comparison;
this ticket does not assume that every possible future failure is preexisting.
All tests actually executed in BUG-255's final matrix pass.

BUG-253/256/257/258 remain done; BUG-254 stays specifying with its native-Windows
blocker. Existing uncommitted source/evidence is preserved. No commit, push,
daemon restart, credentials, gate/security-policy changes, new sessions/subagents,
model substitution, external provider calls, SDK installs or cache deletion.
Disk is monitored between scoped builds; approximately 11 GiB remains, above
the strict 6 GiB floor. No native Windows result is claimed.

## Native completion

Native `update-work-unit-status` accepted **validating → done** after all evidence
was attached and the feature was tagged `@done` with `@wip` removed. The completion
reason records the single successful informed-authorized implementing retry,
four green acceptance cases, 112 distinct scoped passes, successful quality
checks, direct review, unrelated limitations and preserved uncommitted state.
No lifecycle gate was bypassed and no additional implementation retry was made.
