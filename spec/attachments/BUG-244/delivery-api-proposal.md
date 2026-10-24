# Shared delivery API proposal

## BUG245 streaming archive handoff (authoritative update)

Read `spec/attachments/BUG-245/archive-projection-coordination.md`; its requested
disk-backed historical prefix is now installed. New method for BUG245 worker:

```
BackgroundSession::seed_archival_transcript<I>(&self, chunks: I)
    -> Result<bool, TranscriptError>
where I: IntoIterator<Item = Result<(StreamChunk, Option<TranscriptOriginal>), TranscriptError>>
```

It consumes one projected chunk at a time into an anonymous temporary file,
retaining only byte offsets, lengths and original coordinates. Preparation runs
outside the output-buffer lock; installation is atomic and preserves live suffix.
Repeated installation returns Ok(false) without consuming the iterator. Archive
errors do not install a partial prefix. BUG245 should REPLACE the final legacy
seed call, not invoke both seed methods (seeding is intentionally once-only).
The final call site remains BUG245-owned; delivery will not edit it again.

`TranscriptFragment.original: Option<TranscriptOriginal>` is installed with
`TranscriptOriginal { turn_index: usize, message_id: String, timestamp: String,
chunk_index: usize }`. Runtime fragments use None. Assembler checks continuity
and exposes `original() -> Option<&TranscriptOriginal>` for the most recently
completed event. All restoration uses the disk-prefix seed path; no fallback
source is introduced.

`TranscriptAssembler::from_cursor(cursor) -> Result<Self, TranscriptError>` starts
inspection at an arbitrary event-boundary cursor (byte_offset must be zero).
Use this for `/archive <event-sequence>` rather than a default assembler, whose
initial expected sequence is zero. `original()` exposes the completed metadata.

`resident_transcript_chunks() -> usize` is installed and counts only live Vec
entries. Build errors now persist as typed JournalUnavailable page failures,
without installing partial archive data; a successful seed retry clears the
archive error. Clear during preparation invalidates installation by generation.
BUG245's production call is now connected, so the temporary Vec seed API has
been removed; the single disk-prefix path is authoritative for all restores.

Legacy `get_buffered_output` will reconstruct requested historical chunks from
the ephemeral prefix on demand, rather than retaining them in its live Vec.

BUG245 integration update: its lazy archive iterator is now wired in
session_restore and before model-envelope loading in resume_session. Delivery
owner follow-ups requested in archive-projection-coordination.md: expose live
`resident_transcript_chunks()` count for bounded-storage evidence, and remember
archive-build errors for typed page failure until a successful retry. Missing
covered originals must not become an empty successful journal after failed resume.

Delivery worker status: executable RED is confirmed in
`/tmp/bug244-restore-red2.log`: repeated restoration fails its exact-history
assertion at line 259 (0 passed, 1 failed). All API/delegate code is installed;
fragment unit tests are green. `session_restore.rs` is NOW HANDED BACK to BUG245.
Its final replay.chunks rebroadcast loop is replaced by
`session.seed_transcript(replay.chunks);`. Delivery will not edit that file again.

BUG245 coordination request: read
`spec/attachments/BUG-245/archive-projection-coordination.md` before closing
delivery work. The sessions-only BUG245 owner needs journal source handoff and
an RPC-owned optional `TranscriptFragment.original` identity/timestamp field.
Current Vec-of-chunks plus Vec-of-JSON retention is not bounded historical
hydration. BUG245 can implement the separate archive reader and disposable
disk-backed prefix, but must not edit RPC types under its ownership restriction.

## Exact implementation contract (delivery worker)

`get_transcript_page(session_id: SessionId, cursor: Option<TranscriptCursor>,
snapshot_end: Option<u64>, max_bytes: u32) -> Result<TranscriptPage, TranscriptError>`
is additive through SessionManagerHandle, tarpc and both FspecBackend delegates.
Types exported by codelet_rpc_types:

- `TranscriptCursor { generation: String, sequence: u64, byte_offset: usize }`.
- `TranscriptPage { generation: String, snapshot_end: u64,
  fragment: Option<TranscriptFragment>, next_cursor: TranscriptCursor, complete: bool }`.
- `TranscriptFragment { sequence: u64, byte_offset: usize, total_bytes: usize,
  json_fragment: String }` contains UTF-8-safe serialized StreamChunk JSON.
- `TranscriptError` is a serializable typed error (missing session, generation,
  cursor, snapshot, byte limit, unavailable journal, invalid payload).

First request uses None/None. Carry next_cursor and snapshot_end while !complete.
After complete, continue using next_cursor and snapshot_end None to pin a new
exclusive snapshot end. Every page serializes within max_bytes (1024..65536).
One page contains at most one event fragment. Reassemble a whole event before
deserializing StreamChunk; apply sequence once, never append raw live chunks.
Cursor zero is the first event; byte_offset zero is an event boundary.
Generation changes on runtime replacement or clear. Invalid cursors fail closed.

Backend error wrapping uses `BackendError::Transcript(TranscriptError)` and
`BackendError::TranscriptTransport(String)` instead of introducing anyhow into
the new public method. Planned `TranscriptAssembler` in rpc-types exposes
`accept(&TranscriptPage) -> Result<Option<StreamChunk>, TranscriptError>` and
`cursor() -> Option<&TranscriptCursor>`; it validates fragment offsets and cursor
continuity, ignores already-consumed page retries, and emits only whole events.
It retains at most one incomplete event. Parent may use it directly per tab.

Current RED retries initially hit a concurrent missing `MessageStore::store_with_id`
compile blocker; corrected baseline test fixes Result equality without changing
the production path. Final retry is `/tmp/bug244-delivery-red3.log`. Earlier actual
assertion RED for page byte bounds and invalid-request failure exists in
`/tmp/bug244-red-retry.log` (other assertions in that older run had test-helper
issues and are not claimed as behavior evidence).

Delivery types, assembler, journal and all delegates are now wired. The standalone
`transcript_fragments` target passes 3/3 in `/tmp/bug244-fragments-green.log`.
The subsequent integration compile blocker is agent-loop/agent_loop.rs:403,
`StreamChunk::error` requires an owned String, not a string slice. This belongs
to the BUG246 concurrent worker; delivery does not modify that file.

The journal intentionally returns at most one event fragment per page; 2600
small chunks therefore require 2600 inexpensive page requests. Hydration should
continue its async page loop without waiting one UI frame per event. The bounded
fragment serializer does not clone or serialize a whole giant event per page:
JSON is cached once on journal append, and each page examines at most max_bytes.
`StreamChunk` itself does not expose original envelope timestamps; this transport
preserves its JSON unchanged. Any BUG245 archival coordinate/timestamp display
metadata must be supplied explicitly by the separate archive projection rather
than inferred from a new hydration time.

`BackgroundSession::seed_transcript(chunks: Vec<StreamChunk>) -> bool` installs
historical chunks once without broadcasting. The delivery worker replaces only
the final replay.chunks handle_output loop in session_restore, then hands that
file back to the BUG245 parent for archive source selection. No replay_content
or replay_media changes. Parent owns TUI hydration and BUG245.

## Ownership and coordination

BUG-244 owns sessions restore/output buffering, additive RPC/types/backend
delegates, TUI hydration and new helpers/tests. BUG-243 retains exclusive ownership
of core `session_replay.rs` and CLI provider restoration. The later media/events
worker retains exclusive ownership of sessions `replay_content.rs`.

AgentManager messages to the parent and BUG-243 have returned `session_busy` and
were not queued. This note records the coordination request without concurrently
editing their files.

## Smallest complete delivery change

- Add one `get_transcript_page(session, cursor, snapshot_end, limit)` method through
  the existing SessionManagerHandle, tarpc service and both FspecBackend delegates.
- A cursor identifies a runtime session generation and monotonically ordered event
  position. The initial response pins a snapshot end; later responses cannot move
  it. Completed snapshots continue from their final cursor against a new end.
- The existing output buffer remains the runtime journal. Its append and snapshot
  operations serialize with existing session output ordering. Historical restore
  seeds the journal once, without notifying supervisors as new activity.
- The TUI applies only cursor-addressed pages. Raw content pushes are wakeups, not
  another append path. Control requests and transient provider-waiting signals
  retain their existing live path and must not execute during historical replay.
- Periodic catch-up of open tabs recovers dropped wakeups, including server-side
  WebSocket broadcast lag. Reconnect and manual/automatic attach use the same path.
- A per-tab request nonce rejects late pages after close/reopen. One request/page
  is in flight per tab, with timeout, visible scoped errors and retry backoff.
- Count-bounded pages alone do not bound a single very large tool result. The
  contract must either support bounded payload fragments with an intra-event
  cursor or explicitly surface a recoverable payload limitation; silently dropping
  large content is not acceptable. Full payload fragmentation is preferred.

This is additive helper extraction/delegation, not a replacement transport or a
major session/RPC refactor. Existing oversized files receive narrow wiring edits;
new helpers and test files remain below 300 lines.

## BUG-245 shared archival API needed from BUG-243

Please provide a separately named full-original envelope/record export, ideally
`get_session_archival_envelopes(session_id)` or its paged equivalent:

- Original conversational records in manifest order, with original record ID,
  persisted position and timestamp.
- Explicit provider checkpoint exclusion; not blank assistant messages.
- Typed failure for unavailable referenced originals, not filtering them out.
- No compaction summary substitution into original conversation records.
- Read-only: no manifest, token, provider-generation or admission changes.

The current model-replay API remains the sole inference installation path. The
archive export only seeds transcript delivery. Both use the same decoding helper
where applicable, without changing the reserved `replay_content.rs` concurrently.

## Current test-first evidence

Feature: `spec/features/lossless-transcript-delivery.feature`.

Real SessionManager/store tests seed 2,604+ chunks, original text, a tool call and
its result; they exercise embedded and WebSocket services, real subscriber lag,
repeated restoration, visible invalid-request behavior, and the real App renderer.
No production code is changed until the RED integration run completes. The shared
release-slim Cargo artifact lock is currently occupied by other scoped builds.
