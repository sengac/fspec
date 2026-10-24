# TUI delivery integration

Backend audit replacement update: `TranscriptAssembler::from_cursor` is already
implemented at `rust/rpc-types/src/transcript_assembler.rs:13`; the focused
event-boundary test passed in `/tmp/bug244-fragments-final.log` (6/6 target).
Parent inspector can use it now. BUG244 replacement owns new live/cold regression
and BackgroundSession identity; BUG245 owns handle_impl/session_restore guards.
Coordination messages were rejected as busy. Proposed narrow identity API:
`BackgroundSession::requires_model_restore()` is false for fresh live creation,
true only for explicit creation-from-manifest. The manifest path must call
`prepare_manifest_restore()` before publishing/spawning the session. Archive
seeding remains available for cold initialization and explicit restore APIs;
automatic resume guards archival/model/token installation together with the same
identity, without content matching.

Regression is now `rust/fspec-tui/tests/bug244/live_resume.rs`, included in the
already queued `/tmp/bug244-delivery-green2.log` target; it must execute RED before
initialization production edits. It persists real originals, installs actual live
model messages plus an unpersisted suffix, preserves intentionally identical user
messages, checks live counters and unchanged manifest, and resumes from the exact
pre-resume cursor. BUG245 worker may implement its guards after that executable
RED. Do not queue a duplicate delivery target. Existing `persisted_suffix_overlap`
in session_restore still uses message content equality; this is not acceptable as
the live/cold identity fix. Fresh live resume must bypass restoration entirely.

Executable new RED is now `/tmp/bug244-delivery-green2.log`: 8 passed, 1 failed;
first live resume returns `GenerationMismatch` at its pre-resume cursor. Identity
methods and the cold creation marker are installed in BackgroundSession and
SessionManager. Following the documented narrow handoff, delivery installs only
the live-source early return in `resume_session` before archival/model/token
restoration and immediately returns handle_impl to BUG245. Explicit direct
restore APIs remain explicit; no session_restore or archival_replay edit is
needed for manual resume. Backend source is frozen for validation. The final
backend run is `/tmp/bug244-live-init-green.log`, followed by sessions-only strict
Clippy in `/tmp/bug244-live-init-clippy.log`; do not queue a duplicate target.
Parent owns combined TUI/inspector GREEN and work-unit transitions.

Backend ownership audit: a first manual resume of an already-live session must
not prepend persisted originals already represented in its live journal, reset
the generation under an existing tab, or append model replay after its runtime
history. This needs explicit source/model initialization identity and a real live
resume regression, not text equality deduplication. BUG244 backend worker owns
journal identity; BUG245 worker owns resume/session_restore orchestration. Cold
creation-from-manifest and a newly created live conversation must remain distinct.

The TUI consumes the exact cursor/fragment contract at the top of
delivery-api-proposal.md. One page request is in flight per open tab. A random
tab nonce accompanies each response; closed/reopened tabs reject stale responses.
Pages are acknowledged only after validation and complete event decoding. A failed
request leaves the committed cursor and displayed content intact, exposes a scoped
incomplete notice, and retries from that cursor. Broadcast content is a wakeup;
control requests and provider-waiting signals retain their live handling.

BUG-245 worker 738fe807-2baa-41a1-95ca-d755bd72cb93 owns sessions archive projection
and session_restore after the delivery worker's seed_transcript wiring. Coordinate
that handoff before editing the same file. BUG-243 archive owner remains
44135c59-e9f1-4233-a3db-31d97e8ebc4e.

Full original timestamp and manifest record coordinates must survive the archival
projection. If journal events need an additive metadata field rather than timestamp
in StreamChunk, coordinate the shared TranscriptFragment/event schema explicitly;
do not substitute hydration time for the original persisted timestamp.

BUG245 sessions integration now provides `TranscriptFragment.original` through
the delivery-owned `TranscriptOriginal` schema: persisted `turn_index`, original
store `message_id`, RFC3339 `timestamp`, and within-record `chunk_index`.
Delivery assembler exposes the completed identity with `original()`. Parent must
use that timestamp when applying historical events instead of assigning now.
The separate archival iterator decodes one original via Replay::parse at a time,
excludes ProviderState, and seeds delivery's ephemeral disk prefix exactly once.
The reduced model replay remains the only model-message installation path.
