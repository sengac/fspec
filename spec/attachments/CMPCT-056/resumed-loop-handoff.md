# Resumed loop implementation mailbox

Active owner: agent-loop, exclusive `rust/agent-loop/**` plus 053/054/056
specification artifacts. Existing staged and unstaged work is preserved.

CLI owner `93bc5d52-b861-4493-8e05-85a764507dce`: the production
`TurnCompaction::apply(&mut Session, &BackgroundSession) -> Result<bool, String>`
already calls the agreed `Session::commit_selected_compaction(source,
operation_id, content, publish)` API. That CLI API and
`Session::install_selected_preparer` remain absent as of this checkpoint.
Please publish them against the actual selected RigAgent serializer, enforcing
authoritative required-file/reminder obligations and selected model identity.
No synthetic preparation or empty-obligation workaround is being added here.

Gate owner `eb10bc74-4796-422b-a171-ada06fd03b7b`: retain the
`Arc<TurnCompaction>` returned by `register_generate_compaction_handler` and
call its public `apply` after the real tool handoff. This is the production
end-of-stream application entrypoint used by agent_loop; it commits the captured
durable source, publishes the exact generation, and reconciles that identity.
The legacy raw pending-DAG helper rejects uncaptured submissions deliberately.

Baseline command queued on the shared ci-test artifact lock (no separate target):
`cargo test --profile ci-test -p codelet-agent-loop --test cmpct052_durable_submission --test cmpct053_terminal_lock --test cmpct054_terminal_dependencies --test cmpct056_cross_session_locks --test cmpct056_target_identity --test cmpct068_runtime_publication --test cmpct069_token_forwarding -j2`.
Log: `/tmp/cmpct-loop-resumed-baseline.log`. No green claim yet.

Direct owner/parent messages returned `session_busy` and were not queued; this
attachment is the requested coordination mailbox. No board edits, staging or
commits are performed.

First baseline reached loop compilation; owned threshold and graph-future Send
extraction errors are repaired. Second attempt `/tmp/cmpct-loop-selected-red.log`
is blocked by CLI's active `selected_preparer`/`compaction_operation` initializer
edits (duplicate fields in new, missing fields in from_provider_manager). These
are compilation failures, not behavioral red. The selected API now exists;
retry will use it unchanged once those initializer edits settle.

New 056 acceptance tests exercise pending target redirection and interruption
after submission before actual durable application. They use the production
queue, real selected stream preparation and cold durable manifests. No fix has
been applied to those two behaviors before observing execution.

Latest actual execution `/tmp/cmpct-loop-selected-red2.log`: 053/054 all six
tests pass; 056 cross-target positive and 052 positive fail because selected
admission identity compares manifest.provider (the full selection `openai/o3`,
written by SessionManager::create_session_with_id) to current_provider_name
(`openai`). CLI owner: resolve authoritative provider selection centrally;
do not weaken identity checking or rewrite manifests in fixtures. This is a
production-created live/durable session mismatch, not a mock certificate issue.
Pending cancellation and target-redirection tests also execute but are masked
by that later identity rejection. Loop will add precommit target/cancellation
guards and rerun against the same real selected serializer seam.

08:30 resumed checkpoint: `/tmp/cmpct-loop-selected-green.log` is NOT green.
It still rejects052 positive submission/overwrite,056 positive cross-target
commit and pending-redirection recovery with `selected request admission
identity changed`. CLI `selected_compaction.rs` compares `selected.provider`
(current family `openai`) to `source.provider` (production manifest selection
`openai/o3`). CLI owner must centrally reconcile that authoritative selection;
loop does not alter manifests or bypass identity. Cancellation after submission
and changed-target tests pass. Typed overflow fixture corrections are now on
disk and will be rerun. Direct CLI message again returned session_busy.

Caller interruption behavioral RED is now captured in
`/tmp/cmpct056-caller-red.log`: real builder barrier is reached; calling
`caller.interrupt()` without aborting its task times out after3s instead of
returning cancellation. The generation owner now races both captured caller
and target interrupt notifications/flags against the builder, dropping it and
releasing ActivityOwner before any queued submission. Full nine-target loop
execution is queued in `/tmp/cmpct-loop-selected-final.log` with
`--profile ci-test -j2 --no-fail-fast`.

CLI owner: the current disk version of `selected_compaction.rs` still compares
`selected.provider` directly to `source.provider`. Production-created target
sources use selection `openai/o3` while live provider family is `openai`; please
canonicalize the authoritative selection centrally without removing model or
serializer identity checks. Positive loop application remains blocked there.
