# BUG-250 canonical journal archive integration

## Approval and RED

Parent explicitly approved this narrow journal integration after the earlier
audit. The actual failing cold SessionManager test is preserved in
`bug250-cold-display-cargo-red.log`: a durable provider failure was omitted
after its original assistant prefix (zero failures restored, one required).
That executable RED precedes this production repair.

Expanded the feature before adding two canonical export tests, then changed the
shared archival source. The new export tests were queued before production edits
in `/tmp/bug250-archival-journal-red.log`. That filename describes enqueue intent;
their eventual result must be inspected, not claimed as a separate executed RED
if the queue runs them after the repair. The existing executed cold RED is the
behavioral basis for this common-source repair.

## Production integration

`get_session_archival_page` now enumerates the common JournalProjection together
with manifest originals. It exports typed `_transcript` envelopes using each
journal record's original UUID and timestamp. Existing archival replay already
feeds those envelopes through the display-only decoder, so no change to
`sessions/src/archival_replay.rs` or its BUG-245 compaction marker is necessary.
No model replay source, inference installation, token accounting, provider state
or durable message is modified by archival reads.

The canonical paged export therefore includes the same journal projection as
cold display; repairing only the sessions iterator would have left public export
incomplete. Standalone JournalStore export continues to share JournalProjection.
No alternate export store or independent business projection was introduced.

Request/message identities anchor records after the last promoted original in
that request. An unpromoted final request occupies the original suffix boundary.
Interior requests without any original anchor fail explicitly rather than being
timestamp-sorted or guessed into the conversation. Journal order is retained;
inconsistent original-anchor order fails instead of sorting the records.
This is an explicit fail-closed limitation of existing journal identities, not
a fabricated causal relationship for an unanchored historical request.
Anchor validation applies to the shared visible projection, not discarded
aborted-attempt text or internal attempt-control records.

The opaque cursor additionally pins raw journal prefix length and fingerprint.
Later journal appends and later original promotion cannot leak into that pinned
snapshot or suppress its already-selected partial records. Journal mutations or
missing acknowledged records invalidate the source explicitly.
The fingerprint hashes one bounded journal record at a time, without serializing
the entire journal into a second full-history allocation. Original
ArchiveFragment turn indices remain manifest coordinates; transcript records
identify their exclusive original boundary and their separate record kind.
Cursor turn_index is the opaque combined display-record position; end_turn
remains the exclusive original-manifest snapshot end.

Initial coordination to BUG-245 worker was rejected as busy, not delivered.
This repair does not edit that worker's archival replay/media files. The existing
marker and all original manifest indices remain intact.

## Validation and mapping

Two integration scenarios use real stores and the canonical public export API:
backward timestamps across anchored requests, authentic unpromoted content,
original UUID/time preservation, model exclusion, journal-only archives, bounded
escaped UTF-8 fragmentation, append during traversal, later promotion, aborted
attempt filtering and changed-prefix invalidation.
All three feature scenarios have exact single-test/@step/implementation mapping
through the serialized board wrapper. Coverage JSON is below 300 lines.

Parent accepted the earlier BUG-251/252 behavioral and strict-quality evidence
for their unchanged owned sources. Their queued duplicate reruns were therefore
interrupted before compilation to reduce the shared build queue. The focused
session command now writes `/tmp/bug250-cold-canonical-green.log`:

```sh
cargo test -p codelet-sessions --test bug250_cold_terminal_display \
  --test bug250_transcript_replay --profile release-slim -j 2 \
  --no-fail-fast -- --nocapture
```

The canonical core test command is:

```sh
cargo test -p codelet-core --test bug250_archival_journal \
  --profile release-slim -j 2 --no-fail-fast
```

Strict core library CHECK/Clippy is already queued by its owner. Additional
strict test-target Clippy is `/tmp/bug250-archival-journal-clippy.log`; the same
sequential pipeline then runs the original `bug243_archival_pages` CLI regression
into `/tmp/bug250-archival-journal-regression.log`.

No pending command is passing evidence. Leave BUG-250 validating for final parent
audit; no commits, artifact cleanup, alternate profiles or target directories.
