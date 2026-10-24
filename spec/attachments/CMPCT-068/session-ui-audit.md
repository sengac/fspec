# Session and UI acceptance audit

## Completion owner ae474190 — current behavioral baseline

Latest scoped ordering/header probes compile-blocked, not behavioral RED:
`/tmp/cmpct068-background-ordering-red2.log` and
`/tmp/cmpct067-header-provenance-red2.log` both report E0425 in
`cli/src/session/mod.rs:50`, missing `rig::message::Message` import for
`compaction_working_messages`. CLI owner312d7375 please repair your in-flight seam.
Cold restore still compares full saved selection with bare provider name in
`restore_compaction.rs`; pending tests cannot be weakened to bypass this.

`/tmp/cmpct-sessions-completion-baseline.log` executes all ten owned session
targets: reset failure has two behavioral failures; all three cold boundary
targets reject `restored selected identity differs`. CLI owner312d7375: current
`restore_compaction.rs` compares `manifest.provider` (saved selection such as
`anthropic/claude-sonnet-4`) to `current_provider_name()` (bare `anthropic`).
Compare canonical selected provider/model while keeping strict checkpoint model
validation. This is the actual cold-resume blocker, not a fixture expectation
to weaken. Sessions owner is fixing reset propagation and transactional switching.
Changing BackgroundSession::clear_history to return typed reset failure; loop
owner00c6b2ff must handle that Result in agent_loop control clear route rather
than emit cleared success. SessionManager public clear paths will propagate it.

## Active completion handoff — 2026-10-18

**Current integration checkpoint**: CLI restore API landed; sessions now invokes
`restore_compaction_boundary` before provider persistence/replay installation.
Approved `codelet-agent-loop` test-only dependency remains present; providers
test-support is now explicitly enabled in sessions dev-dependencies for gate083.
Scoped fresh-process pending/counter targets are queued in
`/tmp/cmpct068-cold-wired-green.log`. Pending behavioral RED predates this fix;
the queued command is not yet green evidence. Gate owner may use these manifest
seams now without editing production dependencies.

**Resumed 22:32 UTC CLI integration request**: current
`cli/src/session/compaction.rs` still has no `restore_compaction_boundary`.
Please add this typed method now; sessions will call it before provider replay.
Also `reconcile_durable_compaction` still assigns `boundary.input_tokens` to
the live tracker, losing newer saved occupancy. Cold pending probe now asserts
333 occupancy and saved 5030/2900 billing plus1100 reasoning after exact
reconciliation. Both new probes run in a fresh child process, not just a new
manager. Do not claim pending ownership for settled boundaries. See owned
`cmpct068_pending_restore` and `cmpct068_pending_projection` tests.
Sessions dev-dependencies already include agent-loop, futures and provider
test-support; no further gate manifest request remains outstanding.

Resumed owner is finishing implementation, not audit-only. Added workspace
`codelet-agent-loop` and `futures` dev-dependencies in sessions: agent-loop is
test-only for gate083's actual pending application path; futures supports its
shared physical streaming fixture. Production dependency direction is unchanged.

CLI owner request: expose `Session::restore_compaction_boundary(&SessionManifest)`
with a typed result. Validate the authoritative boundary; restore pending identity
and inference barrier when publication is pending; settled restore must not claim
pending ownership. Sessions will call it before replay installation. Current
private fields prevent this in sessions. `cmpct068_pending_restore` verifies
actual `run_agent_stream` zero physical requests, wrong identity rejection and
exact identity reconciliation. Reconciliation must preserve saved counters rather
than overwrite newer saved occupancy with commit-time estimates. Parent and CLI
message attempts returned busy and were not queued; this attachment is the mailbox.

**Behavioral pending RED confirmed**:
`/tmp/cmpct068-pending-completion-red2.log`, 0 passed / 1 failed. Actual
`run_agent_stream` returned success after cold resume despite pending publication;
assertion `pending publication must block the actual inference entrypoint` failed.
Earlier `...completion-red.log` and TUI retry were compilation-only during CLI
module wiring and are not RED evidence. CLI's missing restoration API is now the
critical dependency for this owned fix; see requested signature above.

Owner scope: CMPCT-060, 061, 062, 067, 068, 073 only. No CMPCT-083 edits.
Existing staged work is preserved; no board changes, commits or branches.

## Initial gaps and file plan

- Cold reconciliation currently emits a commit-time token snapshot after saved
  token restoration. That overwrites saved billing/reasoning with zeros and
  replaces newer request occupancy with the original commit estimate.
- TUI conflates the latest request's generation with delivered compaction
  identity. A request arriving before its generation's completion suppresses
  the completion notice entirely.
- The committed-compaction App helper is also not connected to chunk dispatch;
  the existing notice tests should expose this missing production wiring.
- Several sidecar line ranges are stale. CMPCT-068 lacks mappings for durable
  publication and restored context; CMPCT-067 omits the runtime stream test.
- Add feature scenarios, then real cold session and App event-ordering tests,
  then minimally update committed publication, reduction calculation and TUI
  delivery tracking after observing RED. Refresh owned coverage/evidence last.
- Initial Cargo deferred to parent for shared cached-profile baseline.
- A scoped full-slice RED command began rebuilding uncached dependencies in the
  default `rust/target` and was stopped before tests. This is not RED evidence.
  Parent's simultaneous rpc-types baseline was left running. Full-slice tests
  await a coordinated cached target/profile or parent-owned compile slot.

## Recovered prior RED evidence

These are existing local logs, not newly reproduced tests or claims that current
code fails. Compilation-only failures are not counted as behavioral evidence.

| Card | Log | Observed pre-implementation failure |
| --- | --- | --- |
| CMPCT-060 | `/tmp/cmpct060-062-red.log` | 0 passed, 5 failed; wrapped DAG count 0 instead of 1, quoted/mixed content dropped, reminder identity checks absent |
| CMPCT-061 | `/tmp/cmpct061-062-red.log` | 0 passed, 4 failed; unsupported replay silently accepted and structured envelopes rejected during rehydration |
| CMPCT-061 | `/tmp/cmpct060-062-runtime-red.log` | 3 passed, 1 failed; repeated restore duplicated history |
| CMPCT-062 | `/tmp/cmpct061-062-red.log` | 0 passed, 3 failed; invalid counters accepted, live input 0 instead of 101 |
| CMPCT-067 | `/tmp/cmpct067-reducer-red.log` | 0 passed, 3 failed; spend changed fill to 1200 instead of 55; stale fill 95 instead of 10; missing typed event |
| CMPCT-067 | `/tmp/cmpct067-runtime-red.log` | runtime test 0 passed, 1 failed; actual two-request stream occupancy 10000 instead of 8000 |
| CMPCT-068 | `/tmp/cmpct068-wire-isolated-red.log` | 0 passed, 1 failed; committed snapshot event unsupported |
| CMPCT-068 | `/tmp/cmpct068-runtime-red.log` | 0 passed, 1 failed; uncommitted candidate announced completion |
| CMPCT-068 | `/tmp/cmpct068-restored-red.log` | 0 passed, 1 failed; missing restored committed snapshot |
| CMPCT-073 | `/tmp/cmpct073-behavioral-red.log` | 0 passed, 3 failed; saved overrides cached as 123904 rather than 40000/70000; invalid override accepted |
| CMPCT-073 | `/tmp/cmpct073-ci-red.log` | 0 passed, 1 failed, 3 filtered; unresolved explicit profile accepted |

## Coverage interpretation

Sidecars record scenario traceability, not measured execution coverage. Existing
CMPCT-060/061 capture real provider serialization from restored manager history;
they do not prove an authenticated remote provider or the entire live agent loop.
CMPCT-060 includes a fresh child-process restore. CMPCT-062/073 exercise real
SessionManager APIs. CMPCT-067's runtime test runs a scripted two-request stream.
The separate CMPCT-083 owner controls the end-to-end acceptance gate.

## Resumed compile coordination

Parent's shared coordination attachment authorizes scoped RED after the RPC
baseline. Using the shared `rust/target/ci-test` cache and `-j 2` exclusively.
Current commands: RPC snapshot target (`/tmp/cmpct068-audit-rpc-red.log`) then
session restored-counter target (`/tmp/cmpct068-audit-restore-red.log`). No new
target directory or workspace-wide compilation. Production remains unchanged
until each associated behavioral RED is observed.

RPC behavioral RED: 1 passed, 1 failed; spend contaminated reduction was
`-333.3` instead of `66.7`, `/tmp/cmpct068-audit-rpc-red.log`. DTO calculation
now uses the snapshot's declared physical occupancy, awaiting GREEN.

SESSION/TUI COMPILE BLOCKER for runtime owner: session RED compiled dependencies
but fails with CLI E0432/E0433 because `codelet_rpc_types` is not a CLI dependency
(`cli/src/interactive/context_projection.rs:2`, `interactive/output.rs:210`).
Log: `/tmp/cmpct068-audit-restore-red.log`. Runtime message rejected busy, not
queued. Please add dependency in runtime-owned CLI manifest. No session/UI
production edits yet; compilation failure is not behavioral RED.
