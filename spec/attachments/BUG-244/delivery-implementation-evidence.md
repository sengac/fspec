# BUG-244 delivery worker evidence

## Replacement backend audit

The existing `from_cursor` helper and its event-boundary rejection coverage are
present and executable: `/tmp/bug244-fragments-final.log` is 6/6 GREEN;
`/tmp/bug244-fragments-clippy-final.log` finishes successfully with strict warnings.
No replacement implementation of that helper is needed.

A new feature scenario and real regression live in
`rust/fspec-tui/tests/bug244/live_resume.rs`. The test installs actual model
messages, matching persisted originals, intentionally repeated user events and an
unpersisted runtime suffix; it checks counters, durable manifest, exact journal
generation/order and continuation from a pre-resume cursor. The existing queued
delivery target `/tmp/bug244-delivery-green2.log` now includes this regression and
executes RED: 8 passed / 1 failed; the live test returns `GenerationMismatch`
when continuing its pre-resume cursor. This precedes initialization changes.
The replacement now installs explicit BackgroundSession identity and marks the
manifest creation path before publishing the runtime; BUG245 owns the resume
and archival-replay guards. The repeated-restore
test creates a cold runtime explicitly and requires more than 2479 archived
events, so an empty no-op cannot satisfy the assertion.

`/tmp/bug244-delivery-clippy-final.log` reached sessions/RPC successfully but then
fails in parent-external `fspec-tui/src/store/agent_view/token_state.rs:100` with
`clippy::collapsible_match`. This is not a successful final delivery quality run.
Parent pre-inspector target `/tmp/bug244-245-tui-pre-inspector.log` executes
4/4 rendered delivery GREEN and 0/1 archive inspector RED at the expected missing
`/archive` notice. The parent owns that implementation and final combined GREEN.
No work-unit state transition or commit has been performed by this replacement.

The initialized identity additions pass scoped rustfmt checking and `git diff
--check`. Fresh runtime defaults to no model restoration; creation from a manifest
marks restoration explicitly before spawn/publication, and clear resets to fresh.
The parent received the narrow guard handoff via AgentManager after sibling
messages remained busy. No content comparison determines this identity.

Following the documented narrow handoff, BUG244 installs only the live-source
early return in `handle_impl::resume_session` and immediately returns that file
to BUG245 ownership. This bypasses archival/model/token restoration together.
Explicit direct restore APIs are not changed. Backend source is frozen while
`/tmp/bug244-live-init-green.log` runs the full delivery target; successful tests
then trigger sessions-only strict Clippy in `/tmp/bug244-live-init-clippy.log`.

The contemporaneous BUG245 rerun `/tmp/bug245-green2.log` is compile-blocked by
concurrent `agent-loop/src/stream_journal.rs:120` errors (`result` not in scope and
incompatible match arm), not a new behavioral RED. This replacement does not edit
that source or claim the failed compile as initialization evidence.

## Scope

This worker owns the session runtime journal, transport-neutral transcript types,
tarpc service and embedded/WebSocket backend delegates, shared fragment assembler,
and `bug244_transcript_delivery.rs`. The parent owns TUI hydration, rendered
transcript tests, BUG245 archival source selection and work-unit state changes.
No replay_content/replay_media, CLI provider or fspec-tui app/store code is edited.

## Contract

`get_transcript_page(session_id, cursor, snapshot_end, max_bytes)` returns a typed
`TranscriptPage` or `TranscriptError`. Cursor generation identifies the current
runtime journal; sequence and UTF-8 byte offset address the next fragment. An
exclusive snapshot end is carried unchanged until completion. Reuse the final
cursor with no snapshot end to catch up subsequent output. Page JSON is bounded
to the requested 1024..65536 bytes, including escaped payload and cursor metadata.
Large events span pages without truncation. Appends cache their JSON once; page
construction only examines a bounded slice, not the complete event payload.

`TranscriptAssembler` validates continuation, emits only complete StreamChunks,
ignores previously accepted pages, and preserves its cursor on malformed input.
The new backend method exposes typed `BackendError::Transcript` and explicit
transport failures. Unknown sessions and invalid cursors never become empty data.
The tarpc method is appended to preserve existing request variant indices.

The BUG245 streaming handoff adds `seed_archival_transcript(iterator)` and an
anonymous-file historical prefix. It retains only offsets/lengths and original
coordinates, not historical StreamChunk or JSON payload vectors. Preparation runs
outside the output lock, and install checks the captured generation atomically.
Failures discard the temporary prefix; repeated seeds do not consume the iterator.
Runtime suffixes remain live-only. Original turn/message/timestamp/chunk identity
travels in TranscriptFragment.original and remains available through assembler.original().
`TranscriptAssembler::from_cursor` supports bounded arbitrary-event inspection.

`BackgroundSession::seed_archival_transcript` installs historical chunks once,
preserves the runtime suffix and does not publish restored content to either
supervisor or cross-frontend broadcasts. `clear_history` rotates journal generation.
`resident_transcript_chunks` counts live Vec entries only. Build failures remain
visible as typed page errors until a successful retry clears them.
`session_restore.rs` was handed back to BUG245 after initial seed wiring.

## Executable RED

`/tmp/bug244-restore-red2.log`:

```
cargo test --profile release-slim -j2 -p codelet-fspec-tui \
  --test bug244_transcript_delivery repeated_restoration_does_not_rebroadcast_history
```

The real saved-session test executes and fails the exact-history equality at
line 259: 0 passed, 1 failed, 4 filtered. The subsequent implementation replaces
the replay.chunks broadcast loop with one-time journal seeding.

Earlier genuine payload-limit and missing-session assertion failures are in
`/tmp/bug244-red-retry.log`. Other assertions from that earlier log have helper
issues and are not claimed as behavior evidence. Initial current retries were
compile-blocked by concurrent store_with_id/provider/agent-loop edits; these are
not described as executable RED.

## Validation

- `/tmp/bug244-fragments-final.log`: fragment assembler target, 6 passed.
- `/tmp/bug244-fragments-clippy-final.log`: strict Clippy rerun after replacing test expects with propagation.
- `/tmp/bug244-delivery-clippy-final.log`: strict library Clippy rerun after fixing three method-call closures.
- `/tmp/bug244-delivery-green.log`: compile-blocked by concurrent agent-loop journal/persistence edits, not a behavioral failure.
- All builds use the existing release-slim target with -j2; no alternate target,
  profile overrides, commits or done transitions.

The full delivery target asserts both real transports, exact long-session order,
fixed snapshots with appends at every page boundary, retry deduplication, a large
Unicode tool result, invalid generation/sequence/byte limits/missing sessions,
and actual >256-event subscriber lag followed from a nonzero saved cursor.

The integration target also includes disk-prefix tests for original metadata,
large Unicode events, simultaneous live output during iterator consumption,
idempotent non-consumption, rollback/retry, and cached historical Done counts.
BUG245 supplies original coordinates; delivery does not fabricate hydration times.
