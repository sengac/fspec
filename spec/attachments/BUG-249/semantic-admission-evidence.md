# Semantic journal admission finish

Status: **validating**. Parent owns the final independent done audit. No agent-loop
source, schema, provider history, or model replay changes were made here.

## Accepted invariant and ACDD chronology

Parent explicitly confirmed that BUG-249 predispatch acknowledgement must guarantee
a readable truthful lifecycle. Added three scenarios to
`transcript-journal-integrity.feature` before the corresponding tests:

1. Reject missing-request ToolStarted, duplicate ToolStarted, and cross-attempt
   ToolCompleted before acknowledging or publishing another record.
2. Refuse recovery and further admission over persisted invalid lifecycle records,
   preserving every original byte as evidence.
3. Reopening and retrying each immutable requested/started/completed record after
   completion preserves exact IDs, timestamps, bytes, and one authentic result.

All three invalid cases execute before assertions, so a failure on the first
case cannot conceal unexecuted duplicate-start or cross-attempt cases.

The real full-source RED command was:

```sh
python3 /tmp/bug249-core-finish-run.py core/tests/bug249_journal_admission.rs
```

`/tmp/bug249-semantic-lifecycle-red.log` records **3 passed, 2 failed**. All three
invalid candidate cases returned `Ok(())`, changed history, and made projection
unreadable. All three persisted corruption cases permitted read/admission rather
than reporting a lifecycle failure. The valid idempotent retry case already
passed. An earlier attempt using an outdated dependency fingerprint failed to
compile and is not behavioral RED; the runner now resolves the current compiled
core fingerprint and fails explicitly if a matching dependency is unavailable.

Only after behavioral RED, `transcript/store.rs` reuses
`JournalProjection::from_records` under the existing append lease, before staging
or synchronizing the candidate record. New records are validated together with
persisted history; an existing immutable retry is not applied a second time.
Public `read` also validates persisted lifecycle before returning records.
Projection's lifecycle rules are reused unchanged, not duplicated or relaxed.

## Executed GREEN

```sh
python3 /tmp/bug249-core-finish-run.py core/tests/bug249_journal_admission.rs core/tests/bug249_transcript_journal.rs core/tests/bug249_journal_integrity.rs core/tests/bug250_transcript_outcomes.rs core/tests/bug250_transcript_lifecycle.rs
```

`/tmp/bug249-semantic-lifecycle-green.log` records **18 passed, 0 failed**, exit 0:
admission 5/5, transcript journal 4/4, ordering/integrity 2/2, terminal outcomes
4/4, session lifecycle 3/3. This recompiles the entire current core crate against
matching existing release-slim dependency artifacts, then executes tracked
integration tests with the real shared filesystem helper crate. It is
supplementary evidence, not a replacement claim for canonical Cargo completion.

The canonical target was already invoked and remains queued; no duplicate Cargo
build was launched:

```sh
cargo test --profile release-slim -j 2 -p codelet-core --test bug249_journal_admission --no-fail-fast
```

Its output remains `/tmp/bug249-core-admission-red.log` (the filename reflects
original enqueue time, not the eventual result). At recording it contains only
an artifact-lock wait, so no canonical GREEN is claimed.

## Quality and traceability

- `store.rs`: 236 lines; `bug249_journal_admission.rs`: 253 lines.
- Scoped rustfmt passes. No redundant production comments or lint suppressions.
- Journal-integrity coverage is **7/7**, exactly one test mapping per scenario.
  The three new lifecycle mappings include both the store admission/read anchors
  and the reused projection lifecycle implementation. Shifted admission-test
  spans and session-export implementation anchors are refreshed through the
  serialized board wrapper.
- The full-core strict Clippy command
  `python3 /tmp/bug249-clippy-direct.py core codelet_core` now exits **1** due to
  an unrelated concurrent `clippy::large_enum_variant` at
  `rust/core/src/persistence/archive/journal.rs:6`. Exact output is
  `/tmp/bug249-semantic-lifecycle-clippy.log`. That owner must resolve it; no
  fallback, suppression, or out-of-scope archive change was made here.

No normal caller invalid sequence was revealed by these core integration tests.
They do not newly claim to validate the complete agent-loop caller graph; that
source was intentionally untouched. Parent retains the wider caller/replay audit
and all other card done transitions.
