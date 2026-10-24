# CMPCT-068/083 active coordination

Owner: `ea95c81e-72c9-4aa2-942f-e1eecac41c08`.

068 currently has DTO/background-cache/TUI projection tests but **no runtime
CommittedCompaction producer**. Legacy apply_pending_dag_and_emit still publishes
CompactionComplete before durable ownership. I am adding a behavioral producer
test and will integrate one generation-bound event at the durable publication seam.

083 requires a fresh OS process, not clearing process-global caches. I am adding
the cross-process gate independently while durable 055/059 and selected-request
071 APIs are completed. Cold restore must check source originals, exact summary,
ordered tool suffix, committed generation/model/checkpoint, estimated occupancy
and no extra promotion. Provider physical requests must use each actual adapter,
not serialize a synthetic common envelope.

Requests to durable owner `7e9bffc3` and admission owner `663f0f7e` returned
session_busy (not queued). Please expose concrete APIs in your handoffs promptly.
Need durable commit/publish reconciliation identity and selected physical request
preparation usable without live credentials. No completion claim until integrated
runtime and failure matrix pass. No commits.

Concrete producer integration plan: add `codelet_sessions::committed_publication`
with `context_snapshot(manifest, original_tokens, trigger_threshold,
context_capacity, reconciled)` returning a checked `CommittedContextSnapshot`
from the actual durable boundary. The runtime durable publisher should emit this
single `StreamChunk::CommittedCompaction { snapshot }` through handle_output,
not emit legacy CompactionComplete plus unrelated TokenUpdate events. 068 owner
will implement/test this function; durable owner should call it at successful
promotion and at explicit reconciliation with `reconciled = true`.

Current test runs: `/tmp/cmpct068-runtime-red.log` and
`/tmp/cmpct083-cold-red.log`; compile blockers are NOT behavioral red evidence.
The gate currently exercises Anthropic only; five other built-in/contract rows,
OpenAI and the negative matrix remain required. Coverage mapping alone must not
be treated as completion.

068 snapshot producer is now available as
`codelet_sessions::committed_publication::context_snapshot(manifest,
original_tokens, trigger_threshold, context_capacity, reconciled)` with a typed
error for missing/incompatible boundary or unusable limits. It derives generation,
coverage and estimated occupancy from the actual committed manifest, excluding
lifetime output spend. Durable runtime publisher must call this and emit the
single `CommittedCompaction` chunk. Sessions depends on CLI, so CLI should return
the committed manifest to agent-loop rather than importing sessions.

Confirmed actual behavioral red `/tmp/cmpct068-runtime-red.log`: legacy
`apply_pending_dag_and_emit` announces an uncommitted candidate and changes
counters. Durable/079 owners should replace that path with durable commit before
returning a manifest, not just suppress its event. Existing inject_summary_handler
is >300 lines, so extraction approval requested before substantial edits.

Continuation: `context_snapshot` is implemented, but no
`SessionManager::publish_committed_boundary` exists yet. Runtime owner should
call `context_snapshot` on the promoted manifest using effective threshold and
capacity, then `BackgroundSession::handle_output(CommittedCompaction)` exactly
once. I own adding the cold restore reconciliation projection in a separate
sessions module, avoiding the061-owned session_restore.rs. 079/durable owner
must replace legacy `apply_pending_dag_and_emit`; do not mark068 complete while
that false-success route remains. Current publisher check log is
`/tmp/cmpct068-publisher-current.log`.

Current retry `/tmp/cmpct083-cold-current.log` is blocked by duplicate
`pending_compaction_commit: None` in CLI Session::new and missing initializer
in from_provider_manager during055/059 active edits. Test equality compilation
was repaired locally via serialized comparison; this is not behavioral red.
Cold restoration will project the committed boundary from a new module called
at the end of token restoration (one scoped call in061-owned session_restore).
