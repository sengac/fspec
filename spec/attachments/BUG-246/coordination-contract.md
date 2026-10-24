# BUG-246 coordination contract

## Integration ownership

BUG-246 worker `abfb73c7-e0d0-47a0-b088-f216558d98a3` owns spool admission,
retry and stable envelope persistence APIs. BUG-249/250 worker
`4b652e42-041e-46b4-8ddb-642bfe9977a5` has parent approval for narrow
`background_output.rs` and tool-dispatch admission wiring, transcript-only replay,
and the core transcript modules. Coordinate any additional edits to the spool or
`persist.rs` with BUG-246. Preserve the structured media writer and replay helper.

User approval: `spec/attachments/BUG-242/architecture-approval.md` and parent
explicitly approves narrow persistence-only extractions and new conversation commit helper.

## MessageStore primitive requested from BUG-247/248 owner

```rust
pub fn store_with_id(
    &mut self,
    id: uuid::Uuid,
    role: &str,
    content: &str,
    metadata: std::collections::HashMap<String, serde_json::Value>,
) -> Result<uuid::Uuid, String>
```

Under the real writer lock: recover/reconcile index first; an existing identical
immutable payload returns the same identity and acknowledges durability; a
conflicting payload fails; otherwise append with the supplied ID. Propagate
recovery/read errors, never treat unreadable data as absent. Cover append-before-
index-failure and failed acknowledgement using actual filesystem faults.

BUG-246 owns NEW core `conversation_commit::append_message_idempotent` using that
primitive and `update_session_manifest`, adding the reference only if absent.
BUG-246 does NOT modify MessageStore write/index/maintenance modules.

## Active shared-file ownership

Active journal worker `4b652e42-041e-46b4-8ddb-642bfe9977a5` has permission to edit
`background_output.rs` narrowly for journal admission, stable assistant/result
identity call sites and terminal lifecycle integration. BUG-246 owns `persist.rs`,
`persist_state.rs`, `persistence_retry.rs` and `persistence_retry_io.rs`.
Journal recovery records use role `transcript`, empty content, and metadata
`_transcript` containing a `JournalRecord`. Commit validates the session and
record identity before appending to `JournalStore`, never to conversation originals.

Stable entrypoints are `persist_assistant_message_identified(session, provider,
id, content, stop_reason, reason)` and `persist_tool_result_identified(session, id,
tool_call_id, call_id, content, is_error)`, both returning `Result<(), String>`.
Both entrypoints are now implemented in `persist.rs`; journal integration can
call them immediately. The transcript commit branch is now implemented too,
including session/record identity and empty-content checks. Supplemental compiler RED for the missing imports is
`/tmp/bug246-identity-compiler-red.log`; Cargo behavioral RED is still queued
behind the shared release-slim artifact lock. Supplemental artifact-version
errors in that compiler log are not behavioral evidence.

Capacity regression target is `bug246_result_capacity`. A genuine tool result
must still enter original ownership if journal admission/publication is blocked.
The shared owner reserves at most three additional in-process envelopes
(one blocked journal event, assistant invocation and captured result), with a combined 16 MiB bound. They
are not durable admissions and must report shutdown unsafe. Recovery drains
admitted envelopes before promoting these retained originals; it never evicts.
The journal event gate must not return early and discard an authentic ToolResult.
Further dispatch remains refused until recovery succeeds.

Journal admission failures must gate publication/dispatch, not discard original
ownership: retain authentic Text/Thinking blocks even when not published, settle
accumulated assistant originals on Done/Error/Interrupted even if terminal journal
admission fails, and capture authentic ToolResult regardless of journal failure.
The existing `bug246_recovery_admission::unavailable_journal_retains_original_ownership`
asserts explicit recovery creates the assistant original after dropping its output
owner, not merely a transcript event. Both originals and journal events retain
the same message identity so the journal projection can suppress promoted data.

Shared build note: `/tmp/bug242-integrated-check.log` caught a journal initializer
in `LoopDetectionWiring::default` (`session` undefined) and missing from the
`BackgroundOutput` initializer. BUG-246 moved those two initializer lines to
unblock the queue; the active journal owner retains all remaining integration
ownership. BUG-246's malformed UUID patch locations caught by the same check
have also been corrected in `persist.rs`.

## Agent APIs

```rust
pub fn persist_user_content(session_id: &uuid::Uuid, content: Vec<UserContent>) -> Result<(), String>;
pub fn retry_pending_persistence(session_id: &uuid::Uuid) -> Result<(), String>;
```

BUG-251 uses `persist_user_content` for authentic structured input; parent
coordinates its user call-site fail-closed gate. An error never licenses provider
dispatch. Explicit recovery archives accepted pending input; it does not execute
that input. Resubmitting input is a separate deliberate action.

One bounded per-session private recovery directory retains ordered versioned
envelopes with stable IDs. Atomic record admission precedes archive commit; an
acknowledgement removes only confirmed records. Recovery never dispatches a tool
or lifecycle hook. The same mechanism is reserved for BUG-249/250; no new
independent recovery store is authorized.

## Initial RED target

`cargo test --profile release-slim -j 2 -p codelet-agent-loop --test bug246_persistence_recovery`

Tests: `assistant_content_survives_a_temporary_archive_failure`,
`an_invocation_failure_cannot_orphan_its_authentic_result`,
`retrying_a_captured_result_never_repeats_its_side_effect`.
Output: `/tmp/bug246-red.log`.

Parent/core busy-message delivery was rejected; this file provides the exact
contract for concurrent inspection. Additional ownership-boundary scenarios are
written before their corresponding production implementations.

## Ownership and acknowledgement states

- Pending: original caller or retained in-process owner holds the stable envelope;
  it has not been acknowledged by the archive.
- Retryable: an I/O boundary failed; preserve the envelope and its source order.
- Acknowledged: both durable message and manifest association are confirmed;
  removal from the recovery spool is now permitted.
- Promoted but unacknowledged: the manifest was renamed but a later barrier or
  spool acknowledgement failed; reconcile the existing identity, never rollback.
- Permanently failed: incompatible recovery schema, identity collision, or full
  admission capacity requires explicit repair; never evict an accepted envelope.

Three immediate archive attempts are bounded; exhaustion requires visible scoped
recovery and prevents a new inference turn. Retry itself never runs a tool or
hook. Known authentic results are not replaced by cancellation or inferred
success. If even the recovery directory is unavailable, retain bounded original
ownership in memory and warn that terminating the process before recovery cannot
be made durable. No storage architecture can claim durable admission when all
writes fail. BUG-249 must journal before acknowledging displayed prefixes and
before tool dispatch rather than retroactively promising crash durability.

Recovery envelopes preserve original UUIDs and timestamps. Diagnostic errors
contain only scoped operation/state information, never envelope content or raw
provider/tool payloads. Recovery data uses the established private data directory
with owner-only directory/file permissions. Pending records count as reachable
until promotion; cleanup integration must preserve their referenced media.

## Completion audit lifecycle coordination

The current caller worker implemented the specified three retained ownership
slots and authentic-original capture after journal refusal. Direct public
admission never substitutes an empty state for a failed read, and recovery file
ordering/identity corruption fails explicitly. Focused archive commit code now
lives in `persistence_retry_commit.rs`.

BUG-250's core owner has added transcript deletion to SessionStore::delete_owned.
The caller regression `bug250_deleted_recovery` also demonstrated retained
memory/disk ownership could recreate a removed transcript after that operation.
Transcript recovery now requires an existing manifest before appending. This
repairs explicit retry after completed deletion; it is not evidence that a
concurrent delete/retry race is serialized.

Parent/core lifecycle completion still needs coordinated recovery-spool cleanup
and an owned transcript append boundary. Merely deleting the recovery directory
does not clear the agent's retained in-process records. Merely calling
load_session before append is not an atomic deletion/append ownership lease.
Do not add an inverse lock order: retry currently holds the recovery-owner gate
and recovery-file lease before committing, while core deletion holds the session
writer lease. A core-owned non-mutating live-session append operation or another
explicitly approved consistent ownership mechanism is needed for the concurrent
lifecycle guarantee; rewriting the whole manifest on every delta is not an
acceptable substitute. BUG-249/250 done status remains parent-coordinated.

## Integrated ownership resolution

The preceding lifecycle warning describes the intermediate implementation.
Current recovery open/admit/retain/count/retry operations acquire core's shared
`RecoveryLease` and check committed session ownership before mutating pending
state. Explicit absence retires memory and disk; unreadable manifests preserve
the authentic unacknowledged original. Canonical transcript append owns its
session transaction, without a redundant check-then-append manifest read.
The 18:06 `/tmp/bug246-250-final-integrated-cargo.log` executes both BUG-246
ownership tests and seven BUG-250 retirement parents successfully, including
active ownership and real cold recovery. This resolves the caller coordination
gap without adding a second lock implementation or inverse lock order.
BUG-249/250 status still belongs to their assigned owner.
