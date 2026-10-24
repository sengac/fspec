# Incremental transcript architecture

## Active finish ownership

Replacement worker owns transcript modules, stream_journal, narrow BackgroundOutput
and CLI StreamOutput/handle_tool_call gate, and display-only replay_content hook.
Core persistence module registration is now present. Shared admission uses BUG246
stable identified APIs exactly as specified in BUG246 coordination-contract.
Ordering/integrity feature and behavioral regression target:
`cargo test --profile release-slim -j2 -p codelet-core --test bug249_journal_integrity`.
The full archive exporter remains BUG243-owned; journal export integration is
requested as a separate transcript projection, not model originals. Session
deletion must call JournalStore deletion and BUG246 spool deletion. Requests and
implementation evidence are recorded here when busy-message delivery is rejected.

BUG243/245 handoff: Replay::parse will accept an envelope containing only
`_transcript: JournalRecord` and produce display chunks only (no envelopes/model
messages). Incomplete text/thinking are authentic deltas; terminal notices are
typed; requested/started tools use inert notices rather than live spinners or
invented cancellation/results. Preserve record.id and record.timestamp as archive
fragment metadata. JournalStore::project(original_message_ids) excludes promoted
partials and aborted attempt text; JournalStore::export exposes exactly these
scrubbed records. Please include this lane in public archival paging and cold
display iterator, not originals/model replay. Parent message rejected as busy.

Approved scope: `spec/attachments/BUG-242/architecture-approval.md`.

The transcript is a private, versioned, display-only lane, not a model-message
envelope. Stable request/attempt/message/call identities and per-request sequence
numbers identify immutable deltas. Each admission uses BUG-246's RecoveryRecord
owner with role `transcript`, empty content and `_transcript` metadata. Its commit
dispatches to `persistence::transcript::JournalStore`; it never appends a dialogue
message. A durable acknowledgement precedes visible publication and dispatch.

Core API module `transcript` exports `JournalRecord`, `JournalIdentity`,
`JournalEvent`, `TerminalOutcome`, `ToolState`, `JournalStore` and projection types.
`JournalStore::new(data_root, session_id)` and `configured(session_id)` open the
lane; `append(&JournalRecord)` reconciles stable identities and confirms file and
directory barriers; `read()` validates version/framing; `project(&HashSet<Uuid>)`
omits aborted text and originals already promoted and retains authentic tool
lifecycle and terminal identity/timestamps. No recovery API accepts an executor.

Terminal messages come exclusively from typed public codes. Untrusted provider
error prose is never archived. Diagnostics mention operation/state only. Waiting,
spinners and progress are not terminal records. Session deletion must delete its
lane; export reads the same public terminal projection. No automatic journal
cleanup is permitted before original-message promotion evidence exists.

## Integrated admission contract

BackgroundOutput and the CLI executor use the approved narrow journal hooks.
The actual executor's ProgressiveOutput and Arc adapters forward the gate.
BUG-246 provides:

- `persist_assistant_message_identified(session, provider, id, content, stop_reason, reason)`
- `persist_tool_result_identified(session, id, tool_call_id, call_id, content, is_error)`
- RecoveryRecord `role == "transcript"` commit dispatch: decode `_transcript` as
  `JournalRecord`, verify record/session/id match, call configured JournalStore append.

Stable message UUIDs are required to suppress crash-after-original-commit duplicate
partial replay without trusting a later journal acknowledgement. The tool result
has its own message UUID; it must not be suppressed just because its invocation
assistant envelope was committed.

Core registration is present. The store reuses messages::durability barriers and
atomic record publication. Session ordinals are in durable filenames, assigned
under the writer lease; recovery validates contiguous session and request
sequences. Timestamps never order the journal. A missing interior delta is an
explicit completeness error, not a successful shortened answer.

## Remaining shared integration ownership

The real session deletion regression is executable RED in
`/tmp/bug250-lifecycle-real-red.log`. JournalStore::delete exists, but its call in
manifest_commit::delete_owned needs the owning worker or parent's narrow grant.
BUG-246 must also remove pending spool and in-memory ownership on session
deletion so a later retry cannot recreate the deleted transcript.

BUG-244: please seed/merge `JournalStore::configured(session).project(original_ids)`
into archival display only, not model replay. New records carry original timestamps;
terminal public messages are `TerminalOutcome::message()` and severity typed.
Tool projection exposes truthful NotStarted/OutcomeUnknown/Completed/Cancelled.
The approved BUG-251 `_transcript` hook is implemented in replay_content.rs;
the image/document branch is unchanged. Cold archive iteration/public archival
paging must feed these records into that hook. These shared source changes have
not been made by the journal worker. Busy-message requests are not treated as
delivered; the parent received the exact need after owned work executed.
