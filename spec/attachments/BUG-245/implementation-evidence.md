# BUG-245 implementation evidence

Final completion supersedes the historical pending statements below:
`../BUG-244/final-acdd-validation.md` records successful current-tree scoped
tests, Cargo check, strict Clippy and independent acceptance/traceability review.
All eight BUG-245 scenarios pass with 100% scenario mappings; no additional
production behavior was changed during final validation.

## Acceptance-first sequence

`archival-model-replay-separation.feature` was created and validated before tests.
Three scenarios cover real compacted fresh-process restoration and physical model
input, missing covered originals, and bounded large archival hydration with identity.
Example Mapping and estimate were recorded through the serialized fspec wrapper.

`/tmp/bug245-red.log` and `/tmp/bug245-red2.log` were compile failures and are not
claimed as behavioral RED. `/tmp/bug245-red3.log` compiled and executed all three
tests before BUG245 production changes: 0 passed, 3 failed. The cold process showed
only synthetic summary and suffix, the missing original returned successful empty
delivery, and archival identity metadata was absent.

## Connected implementation

`rust/sessions/src/archival_replay.rs` supplies a lazy per-original iterator to the
delivery-owned disk prefix. It reads the BUG243 archival API, verifies fragment
continuity, excludes explicit ProviderState records, and delegates conversational
decoding exclusively to Replay::parse. Original position, store identity, timestamp,
and chunk ordinal accompany every projected event. No archival model messages are
installed into inference. Neither reserved replay decoder file is edited by BUG245.

`session_restore.rs` explicitly names model_replay and discards its display chunks
immediately. Its inference installation remains summary plus compatible suffix,
with all existing committed boundary/provider checks. Only original archival
projection seeds transcript delivery. The resume entry point prepares the same
archive before model-envelope loading, allowing missing-original delivery to fail
visibly without replacing it with a summary. Repeated seeding is delivery-owned
and does not consume the iterator twice or broadcast historical events.

The ephemeral prefix storage, original metadata transport, assembler continuity,
and scoped live-memory metric are coordinated with BUG244 in
`archive-projection-coordination.md` and its delivery contract. There is no second
durable archive or independent source ordering.

## Validation in progress

Initial GREEN run `/tmp/bug245-green.log` executes the real cold-process physical
request assertion and the bounded archive scenario successfully. The missing-
original typed-error assertion also succeeds; its additional repair/retry setup
then hits a stale manifest revision because save_session advances the committed
revision. The test now reloads that manifest before repairing the reference.
Final GREEN rerun: `/tmp/bug245-green2.log`, using only shared release-slim artifacts
and `-j2`. No production change was needed for that fixture correction.

That rerun encounters a concurrent compile blocker in
`agent-loop/src/stream_journal.rs:119` (misplaced synthetic-result branch); it is
not a BUG245 behavioral failure. The sessions owner does not modify that reserved
agent-loop implementation. The owning worker has corrected that branch; final
regression execution is queued as `/tmp/bug245-green3.log`.

`/tmp/bug245-clippy.log` passes with `-D warnings` for the scoped integration target,
including the sessions production source. No commit or done transition is
requested or performed.

Scoped feature validation, tag validation, and formatting check pass. All three
sessions scenarios have test and implementation mappings in
`/tmp/bug245-coverage.log`. The validating transition additionally requires the
parent-owned TUI scenario in `archival-transcript-history-access.feature` to gain
its implementation mapping; BUG245 does not modify that owner’s coverage.

## Fresh completion audit, 2026-10-23

The historical pending statements above are superseded by
`spec/attachments/BUG-244/remaining-repair-evidence.md`, final validation gates.
`/tmp/bug245-completion-sessions-green.log` exits 0: five archival/model scenarios,
the cold-empty-failure scenario with two fresh child processes, and both adjacent
terminal/display replay regressions pass. The shared TUI completion run exits 0
with both archive-inspector scenarios passing through both transports. Coverage
is 100% with one test file per feature, and all three BUG-245 features pass
audit-coverage. Compaction markers, terminal fully-covered archives, missing-
original retry, complete bounded payloads and physical model-input separation
are verified, not inferred from compilation.

BUG-245 is validating via fspec, not done. Scoped Cargo check succeeds, but strict
Clippy is blocked by the untouched baseline `token_state.rs:100`
`clippy::collapsible_match` error. The supervisor handoff records that exact gate;
no warning suppression, unrelated production edit, manual work-unit modification
or commit was performed.
