# BUG-244 / BUG-245 TUI integration evidence

## Connected architecture

- `bootstrap.rs` routes content notifications to transcript wake actions. Live
  state/control requests remain separate from archival content application.
- `dispatch_transcript.rs` fetches byte-bounded cursor pages, validates them with
  the shared `TranscriptAssembler`, and applies complete events once. Snapshot
  ends remain fixed until completion; subsequent snapshots catch up the suffix.
- Each tab has an independent nonce and request identity. Closing, reopening or
  clearing a tab invalidates delayed responses. Closing aborts pending page work.
- The run loop polls open tabs to recover missing notifications. A page timeout
  or source error preserves the committed cursor and rendered content, displays
  an incomplete notice, and schedules a retry.
- `transcript_record.rs` associates persisted original coordinates/timestamps with
  resident rendered turns and removes coordinates when their rows are trimmed.
- Manual resume, reconnect and tab reopening share the same cursor consumer.
  Model installation remains the separate backend resume operation.
- Test support shares real temporary-store fixtures, transport construction and
  action draining. The common scripted backend implements the same page contract
  for focused resume/navigation regression tests.

## ACDD evidence

- `/tmp/bug244-restore-red2.log`: executable exact-history equality failure,
  repeated restoration, 0 passed / 1 failed. TUI implementation began after this
  behavioral RED completed.
- `/tmp/bug245-red3.log`: executable archive/model separation failures, 0 passed
  / 3 failed. Covered originals, unavailable original records and metadata failed
  their actual assertions before archive implementation.
- `/tmp/bug245-history-red.log`: dependency compilation failure, not behavioral
  evidence. The TUI library compiled, but agent-loop persistence compilation
  prevented the history-inspection integration test from executing.
- `/tmp/bug244-245-tui-pre-inspector.log`: scoped TUI integration run queued on
  the shared target. The history inspector remains gated on executable RED.

## Validation status

Pending scoped GREEN/check/Clippy and final coverage mapping. Neither card is done
or committed. All Cargo commands use the existing release-slim target and `-j2`.
