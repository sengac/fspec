# Sessions/RPC/TUI active owner status

Owner `a0a9f8e0` preserves inherited production, tests and specifications. No
board, staging, commit or CMPCT-083 gate test edits are made.

## Coordination mailbox

Fresh audit confirms the inherited real SessionManager atomicity regression
`sessions/tests/cmpct061_restore_atomicity.rs` is behavioral RED in
`/tmp/cmpct061-restore-atomicity-red2.log` (four failures at reopened readiness).
`session_restore.rs` still installs committed checkpoint/counters and clears
readiness before malformed retained provider checkpoints are rejected.

Provider owner: expose nonmutating complete-envelope restore validation against
the authoritative committed checkpoint. Reject null/version/protocol/future
metadata without touching live history, routing, writer or readiness. A shallow
ProviderSession clone shares extension Arcs and cannot stage isolated mutation.
CLI owner: coordinate a Session preflight that validates selected identity,
boundary, replay checkpoints and persistence binding before boundary install.
Reblocking after mutation is insufficient. These tests/spec remain reserved.

The inherited real-tool multi-segment CMPCT-067 target remains reserved:
`fspec-tui/tests/cmpct067_runtime_fill.rs`. Latest inspected evidence
`/tmp/cmpct067-runtime-tool-consistent.log` has correct physical occupancy 8000
but only 1700 visible output / 300 reasoning instead of cumulative 3400 / 600.
CLI/core owners must inspect settlement aggregation; assertions are not weakened.

## Stable traceability handoff

Existing 060/061/062/067/068 mapped features and targets remain stable except
the two reservations above. The new background reconciliation target
`sessions/tests/cmpct068_background_reconciliation.rs` and
`background-compaction-publication-reconciliation.feature` are also stable for
traceability owner `bfd53429`. Inherited GREEN is parent 1/1 and fresh child 1/1
in `/tmp/cmpct068-background-reconciliation-green.log`. Gate adoption should use
`BackgroundSession::reconcile_durable_compaction`, not only inner CLI Session,
to refresh the real background committed snapshot without completion replay.

Messages to coordinator/provider/CLI/traceability owners were rejected busy,
not queued. This file is the durable coordination mailbox. Fresh scoped tests
are running in `/tmp/cmpct-sessions-owner4-tests.log`; no current GREEN claimed
until the result is inspected. Available disk at start: 50 GiB.

## Strengthened reserved regression

The atomicity regression now seeds malformed retained checkpoint metadata in
the real message store, exits to an actual child process and reads the replay
through `get_session_message_envelopes`. Its runtime starts ready with distinct
messages/checkpoint/counters/display. Failure must block readiness while keeping
all other state unchanged. The child then repairs the persisted checkpoint and
retries the real restore. This is stronger than the inherited in-memory replay
mutation. The updated feature validates; fresh behavioral RED is pending the
shared Cargo queue. No atomicity production repair has been made.

Package rustfmt check currently fails on inherited formatting in several stable
test targets and unrelated `dispatch_session_lifecycle.rs`; the reserved new
test's own formatting is corrected. Check/Clippy and RPC/TUI targets are queued
using the shared profile, not separate target directories.

## Additional owned replay identity gap

Source audit found `sessions/src/replay_content.rs` constructs generic tool calls
and results without preserving explicit nested `call_id`, independently of the
core blob rehydration gap. New reserved spec
`session-replay-invocation-identity.feature` and target
`sessions/tests/cmpct061_tool_identity.rs` isolate the SessionManager converter
with authentic distinct `fc_saved` / `call_saved` identities, then capture the
next real Responses request. A second case requires null/numeric/empty explicit
invocation IDs to fail atomically. Specification validates; behavioral RED is
queued in `/tmp/cmpct061-session-tool-identity-red.log`. No repair yet.

## Current results and concrete preflight adoption

The invocation-ID converter repair follows genuine two-test behavioral RED.
`/tmp/cmpct061-session-tool-identity-green.log` now runs the repaired converter
and the existing structured replay target; inspect its terminal result before
claiming full GREEN. No cross-owner files were edited.

CLI's proposed `Session::preflight_compaction_restore(&mut self, manifest,
envelopes)` API is accepted for adoption before boundary, writer and provider
restore. Provider's nonmutating `ProviderSession::validate_restore(envelopes,
committed)` must validate every retained envelope and proposed generation without
mutating shared extension Arcs. Sessions is ready to call that preflight once
the owning CLI/provider implementations land; current four fresh-process cases
remain behavioral RED at reopened readiness in
`/tmp/cmpct-sessions-owner4-tests2.log`.

The runtime067 failure is still real in `/tmp/cmpct-tui-owner4-tests.log`:
two physical tool requests produce1700/300 instead of3400/600 visible spend.
CLI's FinalResponse branch retains `streaming_display.current()` whenever an
earlier Usage event made output authoritative, ignoring later aggregate usage.
CLI owns that aggregation repair. Occupancy8000 remains correct.

New reserved `measured-reasoning-cold-session-restore.feature` will exercise
actual physical settlement, compaction and fresh SessionManager token restore
twice; this supplements the core durable reasoning regression rather than
substituting handcrafted saved counters for measured reasoning.

## Recovered exact results and urgent Codex limits root cause

Inspected tool-identity GREEN log: new target2/2 and structured replay4/4 pass.
Current sessions suite has only the four reserved atomicity failures; scoped
sessions/RPC/TUI check finishes successfully. Clippy finishes successfully with
one inherited TUI collapsible-match warning. TUI suite still has only the
reserved067 cumulative-spend failure. No completion is inferred from log names.

Codex fresh-child zero limits have a concrete upstream cause: sessions' shared
`apply_model_selection` calls `set_model_direct(codex, model, None, None, None)`,
which clears registry limits. ProviderManager's Codex limits resolver defaults
both capacity and output to zero (`providers/src/manager.rs`). Thus the correct
shared session creation code receives zero capacity/output/trigger. Provider
owner must restore authoritative Codex limits/registry selection; no invented
session-specific hardcoded fallback is acceptable. Direct handoff rejected busy.
Existing083 cold failure is genuine behavioral RED for this gap. Current disk
headroom25GiB; only bounded targeted shared-ci-test validation remains safe.

## Restore-preflight integration mailbox — 19:05

Provider APIs are now present and behaviorally verified: inspected
`/tmp/cmpct-provider-freshness-preflight-green.log` ends Cargo exit0, including
two nonmutating restore-preflight regressions. CLI can integrate
`validate_committed_restore(generation, metadata, envelopes)` now. Sessions
still awaits `Session::preflight_compaction_restore(manifest, envelopes)` and
will call it before boundary, binding, provider state or runtime mutation. The
four fresh-child atomicity failures remain reserved; readiness blocking after
partial mutation is not an acceptable substitute. Direct CLI message was busy
and not queued. Source check confirms no CLI preflight method yet.

Measured-reasoning test first compile exposed a stale manifest field name,
not behavioral RED. The test now reads committed identity generation and reruns
in `/tmp/cmpct062-measured-reasoning-red2.log`; no production repair is made.

The converter target `cmpct061_tool_identity` and specification
`session-replay-invocation-identity.feature` are now stable and explicitly
handed off to traceability owner `bfd53429`, including sidecar/step documentation.
Its genuine RED2/2 and GREEN2/2 plus structured replay4/4 are recorded above.
Only atomicity061, measured062 and runtime067 remain reserved in sessions scope.

## Measured reasoning fresh-process verification

`/tmp/cmpct062-measured-reasoning-run4.log` finishes with parent1/1 and real
fresh child1/1 GREEN. Two captured physical Responses requests settle measured
reasoning30+7 and billed output100+77 before a real complete-prefix compaction.
The fresh SessionManager restores twice, requiring reasoning37 in live tracker,
background cache and emitted TokenUpdate, input billing200, output billing177,
and unchanged two physical settlement identities. No handcrafted reasoning
counters substitute for measured requests. The first runs corrected compile,
explicit fixture admission limits and complete persisted DAG coverage; they are
not claimed as reasoning behavioral RED. Core's earlier genuine reasoning
RED/GREEN already repaired persistence; no sessions production change was needed.

`measured-reasoning-cold-session-restore.feature` and
`sessions/tests/cmpct062_measured_reasoning.rs` are now stable and transferred to
traceability owner `bfd53429` for actual sidecar/step mapping. Syntax and scoped
tags validate. Atomicity061 and runtime067 remain reserved and failing until
their real cross-owner runtime repairs land. Current headroom24GiB.

## Current scoped quality and independent acceptance inventory

New-target `cargo check --profile ci-test -j2 -p codelet-sessions` with explicit
`--test cmpct062_measured_reasoning --test cmpct061_tool_identity
--test cmpct061_restore_atomicity` finishes successfully in
`/tmp/cmpct-sessions-owner4-new-check.log`. The same explicit targets pass
Clippy in `/tmp/cmpct-sessions-owner4-new-clippy.log`; shared fixture dead-code
warnings and inherited dependency warnings remain, not denied errors.

Focused rustfmt check of `session_restore.rs`, `replay_content.rs`, reserved
atomicity061 and new measured062 passes exit0 in
`/tmp/cmpct-sessions-owner4-current-format.log`. Whole-package formatting is
not claimed clean: the earlier package check includes inherited formatting
backlog and traceability-owner-held stable tests. No cross-owner reformat is
performed to manufacture a clean result.

Inspected completed scoped acceptance logs retain these exact parent counts:

| Sessions target | Passed | Failed |
| --- | ---: | ---: |
| cmpct060_typed_summary | 5 | 0 |
| cmpct061_reset_failure | 2 | 0 |
| cmpct061_structured_replay | 4 | 0 |
| cmpct061_tool_identity | 2 | 0 |
| cmpct061_restore_atomicity | 0 | 4 |
| cmpct062_token_restore | 3 | 0 |
| cmpct062_measured_reasoning | 1 | 0 |
| cmpct068_background_ordering | 1 | 0 |
| cmpct068_background_reconciliation | 1 | 0 |
| cmpct068_durable_publication | 1 | 0 |
| cmpct068_pending_projection | 1 | 0 |
| cmpct068_pending_restore | 1 | 0 |
| cmpct068_restored_counters | 1 | 0 |
| cmpct068_restored_projection | 1 | 0 |

Fresh children are additional real executions, not double-counted parent tests.
RPC `cmpct068_committed_snapshot` passes2/2. TUI coherent-fill3/3,
header-provenance1/1 and committed-projection4/4 pass; actual runtime-fill0/1
fails cumulative output/reasoning. These are selected scoped results, not whole
package or complete-card readiness claims. Remaining production dependencies are
CLI's nonmutating restore preflight and cumulative final display accounting.
