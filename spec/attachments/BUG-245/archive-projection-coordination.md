# BUG-245 session projection handoff

## Live/cold initialization coordination (replacement BUG244 worker)

`from_cursor` is already implemented and 6/6 unit target GREEN in
`/tmp/bug244-fragments-final.log`. New live regression is included in the already
queued `/tmp/bug244-delivery-green2.log`; do not add another delivery build.
Executable RED is confirmed below. Additive API owned by BUG244:
`BackgroundSession::requires_model_restore() -> bool` defaults false for fresh
live runtimes; `prepare_manifest_restore()` marks cold origin before spawn or
publication in `create_session_from_manifest`. Clearing makes the runtime fresh
again. The resume early return for false precedes archival/model/token restoration.
This
prevents prepending represented originals, changing a live cursor's generation,
appending a second model replay, or overwriting live counters. Message equality
must not decide initialization. BUG244 owns the marker and BackgroundSession;
BUG245 retains handle_impl, session_restore, archival_replay. Initialization has
executable RED: `/tmp/bug244-delivery-green2.log`, 8 passed / 1 failed,
live regression returns GenerationMismatch from its saved cursor. BUG244 has now
installed the two identity methods, manifest-creation marker and narrow resume
guard. Direct restoration APIs remain explicit and must not be converted to
content-based no-ops. No session_restore/archival_replay edit is needed for the
manual-resume fix.

Narrow guard wiring notice: sibling messages remain busy and the parent received
the exact guard handoff. BUG244 installs only the live-source
early return in `resume_session` before its restoration block; handle_impl is
immediately handed back afterward. No session_restore or archival_replay change
is needed for manual resume: explicit direct restore APIs remain explicit. This
avoids holding the source fix behind an idle handoff while preserving archive
projection ownership. Parent whole-GREEN remains the completion gate.

Backend source is frozen. Final delivery run is queued in
`/tmp/bug244-live-init-green.log`, followed on success by sessions-only strict
Clippy in `/tmp/bug244-live-init-clippy.log`. Do not queue duplicate backend
targets or change work-unit status before parent combined GREEN.

Owner: 738fe807-2baa-41a1-95ca-d755bd72cb93. Parent owns TUI hydration.
Live messages to BUG-243 and BUG-244 owners were rejected as busy.

Executable BUG245 RED completed: `/tmp/bug245-red3.log`, 0 passed / 3 failed.
Cold process displayed summary plus suffix instead of originals; missing original
returned an empty successful transcript; metadata was absent. BUG245 is implementing.
The sessions-only `archival_replay.rs` consumes core archival pages one record at
a time into delivery's disposable disk-indexed transcript prefix. That prefix
reads at most a page-sized JSON fragment, never a full large event when paging.

Streaming handoff received from delivery-api-proposal.md. BUG245 has connected
`restore_archival_transcript()` to the final session_restore call, using a lazy
one-original-record iterator into `seed_archival_transcript`. No BUG245 spool
remains: delivery owns the single ephemeral prefix store. Model chunks are
discarded immediately after shared decoding; only model messages are installed.

Delivery provides `BackgroundSession::resident_transcript_chunks() -> usize`,
returning only the live Vec count, not reconstructed archived chunks. The
integration scenario asserts that storage bound while preserving on-demand
get_buffered_output semantics. BUG245 does not edit journal/background_session.

Missing-original integration remains the executable-RED page test unchanged.
A narrow resume_session archival seed precedes fetching model envelopes, so a
missing covered original is detected by archival preparation first. Delivery
records build errors without installing a partial prefix; get_transcript_page
returns the typed error rather than empty success. A successful retry clears
that failure. Error strings retain the original missing UUID, making the real
TUI incomplete/error path observable.

Delivery/parent audit item: first resume of an already-live, never-seeded runtime
must not prepend persisted copies of events already in its live journal. Its
source initialization needs to distinguish a cold runtime from an authoritative
live runtime; comparing content strings is not a safe identity policy. Once a
prefix is installed, ordinary appended live suffixes and repeat resume are
covered by the existing once-only seed contract. BUG245 does not modify delivery
ownership or introduce content-based deduplication.

Restore consumes the BUG-243 archive API one record at a time and keeps historical
chunks out of the runtime output buffer. The delivery historical-prefix source
consumes an iterator of original records/chunks without retaining their payloads.
Its ephemeral on-disk projection is rebuilt from authoritative manifest order,
not another persisted truth.

The source must retain original turn_index, message_id, timestamp and within-record
chunk ordinal. Expose them as optional TranscriptFragment original metadata, not
hydration timestamps. Parent can use this metadata for original display time.
The transcript assembler must keep this identity available after event assembly.
ProviderState archive records are excluded explicitly. Only Replay::parse decodes
conversation envelopes. Its messages are discarded for archival projection.

Model replay does not install archival messages into inference. The integration
points are session_restore's final archive seed and resume_session's archival
preparation before model-envelope loading. TranscriptJournal remains BUG244-owned.

Proposed exact metadata: `TranscriptFragment.original: Option<TranscriptOriginal>`
where `TranscriptOriginal { turn_index: usize, message_id: String,
timestamp: String, chunk_index: usize }`. Timestamp uses persisted RFC3339 UTC,
not hydration time. First-fragment metadata is checked by the BUG-245 large archive
scenario via serialized JSON, so existing delivery types still compile for RED.

Three tests are in `rust/sessions/tests/bug245_archive_model_separation.rs`.
Executable RED is `/tmp/bug245-red3.log`; earlier attempts were compilation
failures, not behavioral evidence. Validation uses the shared target, with results
in `/tmp/bug245-green.log` and `/tmp/bug245-clippy.log`.

Clippy passes with -D warnings. Initial GREEN passes cold physical model separation
and bounded archive hydration; missing-original typed failure also works, but its
repair fixture needed a manifest reload before save. Corrected rerun
`/tmp/bug245-green2.log` is blocked by a concurrent syntax/undefined-variable error
in agent-loop/src/stream_journal.rs:119-120, outside BUG245 ownership.
That branch is now corrected by its owner; `/tmp/bug245-green3.log` is the final
regression run in the shared build queue.
