# BUG-258: approved bounded extraction and terminal journaling plan

## Approval boundary

The earlier review withheld approval pending this concrete plan and real red
evidence. On 2026-10-25 the human approved this bounded extraction, conditional
on native Fspec ACDD recording the existing card's scope, appropriate status and
new Markdown attachments before starting: "well you have to do it acdd via fspec..
either update the existing card and set the right work status, and attach new
markdown documentation to that card then start it". This supersedes the earlier
hold; it does not authorize broader redesign or bypassing lifecycle gates.
Record approval, attach this plan and red evidence, add the applied-compaction
budget control, then transition testing to implementing before production edits.

The 300-line rule remains unchanged. `agent_loop.rs` has 1,536 lines, almost all
in one async function; `BackgroundOutput` has 739 lines. Unlike the independent
BUG-257 item split, a source-fragment include cannot safely subdivide this async
function: its input flags, mutex guard, request-owned output, handler registrations,
recovery counters and cancellation decisions cross the candidate boundaries.
Replacing the few publication calls while leaving the touched file oversized is
not proposed. Neither is global interception of every published session error.

## Independent ACDD work completed

Native Fspec records discovery/story/rule/example, the answered approval
question, five scenarios, validation, and the testing transition. Tests were
written without any BUG-258 production change. All five scenarios have test
coverage mappings, but deliberately **no implementation mapping**.

Actual command, from `rust/`:

```text
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug258_outer_terminal -j2
```

Result: **2 passed, 3 behaviorally failed, 1 ignored child-process entry point**.
The ignored entry point is explicitly executed in fresh child processes by the
five parent scenarios; it is not a skipped acceptance case. Exact final output
is preserved in `spec/attachments/BUG-258/red.log`.

| Scenario | Live-path proof | Cold result |
| --- | --- | --- |
| Invalid request preparation | Real limits 4096/4096 rejected before streaming; 0 inference and 0 builder calls; live Error and Idle/Done observed | 0 safe failures, expected 1 |
| Failed outer overflow builder | Real inference overflows; builder is called and fails; outer Error/Done observed | 0 safe failures, expected 1 |
| Exhausted outer overflow recovery | Exactly 4 inference attempts and 3 builders; live recovery-budget error | 0 safe failures, expected 1 |
| Already-journaled inner error | Real provider request receives a deterministic non-overflow rejection | Pass: one durable/cold failure |
| Successful automatic recovery | One builder and two inference calls; original task finishes | Pass: no durable/cold provider failure |

The shared existing automatic-compaction harness runs the real background loop
against loopback-only offline HTTP fixtures. Test setup adds genuine envelope
fields to the harness's two pre-existing original messages and seeds the cached
model catalog; it never fabricates a terminal record. Fresh OS processes create
real SessionManagers and restore the same archive twice, checking display
identity/order, immutable journal/manifest evidence, and absence of safe terminal
prose, raw provider error strings and transcript metadata from model history.
The first fixture-only bare-model-string cold-restore error was corrected before
accepting the final behavioral red. Both passing controls prove cold replay and
the fixture work independently of the three missing-outcome failures.

Test Clippy succeeds:

```text
cargo clippy --offline --profile ci-test -p codelet-agent-loop --test bug258_outer_terminal -j2
```

Only unused members of reused test fixtures warn; no lint was disabled. Both new
Rust test files are below 300 lines. Targeted formatting and `git diff --check`
pass. No external provider is called, and test listeners bind only to loopback.

## Proposed extraction: exact modules and functions

These are approved proposed names, **not new production files already written**. Existing
helper implementations remain the single source of truth. Target every file
below 270 lines to leave room for the eventual focused fix.

| Proposed file under `rust/agent-loop/src/` | Functions/types and current boundary |
| --- | --- |
| `agent_loop.rs` | Keep `agent_loop` as the orchestrator, receiver ownership and explicit turn scope; aim below 220 lines. |
| `agent_loop/state.rs` | `LoopState` owns the existing `mcp_channel_open`, `compaction_retry_input`, `automatic_recoveries`, `loop_abort_retry_count`; `PendingInput` carries the existing fresh/retry/compaction/epoch flags without changing their meaning. |
| `agent_loop/input.rs` | `next_input` moves the retry selection and biased user/supervisor/MCP select from current lines 129-290. It returns input/skip/closed dispositions, not a spawned task. |
| `agent_loop/lifecycle.rs` | `start_session_hooks`, `end_session_hooks` move current startup and input-channel-close hooks. `end_session_hooks` runs once only on the same exit condition. |
| `agent_loop/prompt.rs` | `admit_prompt` moves epoch/cancellation checks, fresh-user budget reset, user hooks and durable user persistence from current lines 292-418; returns dispatch/skip, preserving existing early notices. |
| `agent_loop/routing.rs` | `resolve_turn_routing` moves provider/model/facade/thinking-key selection from current lines 418-480; keep the same short inner lock. |
| `agent_loop/thinking.rs` | `resolve_turn_thinking` moves the current adaptive/non-adaptive thinking resolution from current lines 480-590; no provider/model substitution or new lookup path. |
| `agent_loop/handlers.rs` | `register_core_handlers`, `clear_core_handlers` move pause, fspec, HITL, exec-stdin, search and graph registration/cleanup. Do not introduce a new RAII cleanup policy. |
| `agent_loop/services.rs` | `register_service_handlers`, `clear_service_handlers` move the existing DeepSearch, AgentManager, schedule and compaction-registration calls. Return the existing `Arc<TurnCompaction>` and `CompactorRegistration` explicitly to the orchestrator. |
| `agent_loop/bridge.rs` | `register_bridge_handlers`, `clear_bridge_handlers` move current relay broadcast adapters, input/control/command callbacks, context and bridge handler from current lines 821-1028; preserve current task creation/closure ownership exactly. |
| `agent_loop/providers.rs` | `run_selected_provider` moves the current match at lines 1078-1348, retaining `run_with_provider!` as the built-in dispatch source of truth. Delegate only the large existing arms to the next files. |
| `agent_loop/provider_openai.rs` | `run_openai_turn` moves the existing OpenAI arm verbatim, including tool wrappers, selected preamble, cancellation handles and the one stream entry point. |
| `agent_loop/provider_custom.rs` | `run_custom_turn` and feature-gated `run_stub_turn` move existing custom/stub arms, preserving facade/model aliases and unsupported-provider behavior. Split the stub arm into `provider_stub.rs` only if formatting would reach 300 lines. |
| `agent_loop/reconciliation.rs` | `reconcile_turn` moves pending annotations, `TurnCompaction::apply`, cancellation reconciliation, committed learning extraction and compaction-flag/progress cleanup from current lines 1350-1416; return the existing result/applied/cancelled facts. |
| `agent_loop/finalization.rs` | `settle_turn_result` and `schedule_loop_retry` move current lines 1430-1534: owned overflow recovery, recovery budget, final error/completion decisions, compactor-registration drop point and loop-detector retry. |
| `agent_loop/idle_guard.rs` | Move the existing `IdleOnDropGuard` type/Drop implementation only, retaining its original arming/disarming points. |

The table is a dependency-preserving extraction, not permission to redesign the
runtime or add a new execution engine. If a smaller grouping stays below 300
lines without crossing these ownership boundaries, merge that grouping during
review rather than creating unnecessary abstractions.

## Ownership, cancellation and recovery invariants

1. The orchestrator retains the `session.inner` mutex guard acquired at the
   current line 593 until the same end-of-turn point. Helpers receive
   `&mut Session`; they must not reacquire that mutex. Finalization calls
   `try_terminal_overflow_recovery_owned`, never the public locking wrapper.
2. Create one `BackgroundOutput` at the existing turn point and pass it by
   reference to streaming and finalization. Never allocate a replacement journal
   for the final fallback: request/attempt IDs, sequence, anchor, pending original
   ownership and deduplication must survive the decision.
3. Preserve the supervisor receiver lock's lifetime and release it before turn
   processing. Preserve biased user priority, pending-message decrement, and the
   disabled-closed-MCP flag to avoid busy looping.
4. Preserve both epoch cancellation checks surrounding asynchronous work, the
   interrupted flag/notify handle, and the special loop-abort retry admission.
   Never reset counters for supervisor/MCP/synthetic continuation input as if it
   were a fresh user request.
5. Keep compactor ownership/handlers alive through the outer recovery call.
   Clear the same ordinary handlers at the existing point; drop
   `CompactorRegistration` only after settlement. Keep Idle-before-Done ordering.
6. No early `Error` terminal is admitted while recovery may succeed. An already
   admitted inner terminal deduplicates on the same owner. Immutable completed
   records must not be rewritten to hide premature completion.
7. Before changing terminal handling, add a specific control for the separate
   `applied && automatic_recoveries >= 3` success/compaction branch. Verify
   whether an inner Done has already finalized that journal; if so, review a
   scoped deferred-completion decision rather than silently suppressing the
   later failure or inventing a new request identity.
8. Leave pre-output setup failures and panic-guard semantics unchanged by the
   mechanical extraction; any proposed expansion of durable terminal coverage
   there needs its own explicit acceptance test and review.

## Minimal journaling change after behavior-preserving extraction

Add `BackgroundOutput::finish_outer_failure` in the already-small
`background_recovery.rs` child implementation (currently 48 lines), avoiding an
unnecessary rewrite of the 739-line parent output implementation. The method
routes abandoned final Error and Done through existing `StreamOutput::emit`,
maintains Idle-before-Done, and reuses `StreamJournal::admit` for typed safe
outcomes and retries. Call it only from the abandoned-result and exhausted
post-compaction branches in `settle_turn_result`. Successful recovery schedules
continuation without calling it. Review the deferred-completion risk above
before assuming this adapter alone is sufficient for the applied branch.

No manual journal insertion, parent/global publication hook, synthetic terminal
prose in originals, disabled lineage check, or gate/file-size exception is part
of this proposal. Make extraction behavior-preserving first: its test failures
must remain the same three behavioral reds, and its passing controls must stay
green, before the terminal-routing change is introduced.

## Approved validation commands

From `rust/`, all offline, package/target scoped, ci-test and -j2:

```text
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug258_outer_terminal -j2
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug210_automatic_continuation --test cmpct053_terminal_lock --test cmpct054_terminal_dependencies --test cmpct054_cancellation_cleanup --test cmpct054_bridge_clear_failure -j2
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug211_compaction_budget_recovery --test bug211_explicit_retry_safety --test rpc089_mcp_injection_drain --test rpc087_dispatch_routes_through_recovery_stream_shape -j2
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug246_persistence_recovery --test bug246_recovery_admission --test bug246_journal_promotion --test bug250_terminal_archive --test bug250_request_anchors --test bug250_deleted_recovery --test bug250_recovery_retirement -j2
cargo test --offline --profile ci-test -p codelet-sessions --test bug245_cold_empty_failures --test bug250_transcript_replay -j2
cargo test --offline --profile ci-test -p codelet-agent-loop --test bug253_gemini_invocations --test bug257_fork_continuation --test prov149_codex_reload -j2
cargo check --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-core -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-agent-loop --test bug258_outer_terminal -j2
```

Use native Fspec for feature/tag validation, exact scenario coverage refresh,
coverage audit, evidence attachments and lifecycle transitions. Preserve the
known unrelated source-shape failure documented on BUG-257; do not fix or hide
it as part of this extraction. Monitor the 6 GiB free-space floor between builds;
request guidance instead of deleting caches if that floor becomes unsafe.

## Pre-implementation production preservation baseline

Both files remain byte-identical to HEAD after the independent BUG-258 work:

```text
agent_loop.rs:       0d35b885246a74956a979e078afccd050cb9e0fd
background_output.rs: 0e7b87082a3b7e7aea1b596103cfed45cff622e8
```

At this baseline no BUG-258 production extraction or fix had been made. BUG-253/256/257 completed
work and BUG-254/255 holds are preserved, uncommitted. Free space after the final
red/test-Clippy run: approximately **7.7 GiB**, above the 6 GiB floor.

## Approved execution progress (2026-10-25)

Native Fspec recorded the approval and expanded card scope, attached this
Markdown and the original red log, returned to specifying for the added budget
criterion, then advanced through testing to implementing with all six scenario
test mappings. No temporal-validation override or manual card edit was used.

The added applied-compaction control reaches four inference calls and four
successful builder calls before the budget error. Its final request has **zero
terminal records**, not an already-finalized Completed record. The actual
`compaction_handoff::requires_handoff` success path sets `summary_handoff` and
breaks the inner stream before `emit_done_with_stop_reason`; the outer owner
then applies the captured candidate. Consequently this accepted path needs no
terminal rewrite, replacement identity or deferred-completion redesign. The
new red is alongside the original three behavioral reds and two passing controls
in `applied-control-red.log`.

The approved phase modules are now wired into the existing agent-loop entry
point. The orchestrator retains the same inner mutex guard until turn-scope
exit. Ordinary-handler cleanup retains its prior ordering; compactor registration
is dropped after final settlement, before loop-detector retry scheduling. The
parent `background_output.rs` and journal implementation remain untouched.

Two initial extraction compile errors were corrected by retaining the existing
types (`&Arc<BackgroundSession>` for owned terminal recovery and
`Result<bool, CompactionFailure>` for builder retry). An interrupted baseline
run reached behavioral reds but was not accepted as completed evidence.

At the human's explicit request the supervising caller interrupted this session,
verified no remaining build or patch process, and ran `cargo clean` in `rust/`.
It reported 38,548 files / 25.1 GiB removed; actual free space rose from 6.52 to
30.62 GiB. Source, evidence and other caches were preserved. Resume inspection
confirms BUG-258 remains natively implementing, no active build process and
`git diff --check` passes. A fresh offline ci-test baseline rebuild is required;
the truncated interrupted output is not proof of a completed test run.

The post-clean offline rebuild and baseline completed: four behavioral reds and
two greens (`extraction-red.log`). Review retained the small guarded receiver
select and its local MCP-open flag inline in the orchestrator, with retry and
supervisor formatting helpers in `input.rs`. This permitted smaller grouping
keeps the inner guard explicitly owned by the orchestrator and preserves all
five unchanged RPC-089 source contracts. The reviewed extraction again produces
the same four reds/two greens (`extraction-reviewed-red.log`).

Only after that baseline, `finish_outer_failure` was added to the small recovery
child and called from the two planned settlement branches. All six acceptance
tests now pass (`green.log`); the child helper is explicitly run six times.
Final broad-suite caveats and review results are recorded in
`acdd-validation.md`, without modifying unrelated tests or specifications.

### Live diagnostic compatibility refinement

The unchanged explicit-builder retry regression exposed a related compatibility
gap after the six acceptance greens: that path already acknowledges a
CompactionFailed terminal and publishes a warning; the canonical outer Error
would be suppressed by blanket terminal deduplication, removing the existing
actionable Error chunk. This is not an unrelated source-shape failure.

Return natively to specifying/testing for a seventh real-path acceptance case.
The bounded repair may distinguish one live ProviderError diagnostic after an
acknowledged CompactionFailed terminal in `StreamJournal::terminal`, without
appending or rewriting any terminal. A per-owner flag prevents repeated live
diagnostics. ProviderError-after-ProviderError remains suppressed. Continue using
the same `BackgroundOutput::emit` path; do not intercept global publication or
create another request identity. The journal file remains below 300 lines.

The stronger case also counted both safe failure kinds and exposed the existing
second output allocated inside owned overflow recovery. That helper now borrows
the original turn output; only the independent public locking wrapper constructs
its own standalone owner. Recovery failure and outer diagnostic therefore share
one journal, not two identities. Cold assertions recognize the existing safe
CompactionFailed warning rendering as well as ProviderError Error chunks. All
seven acceptance cases pass after these bounded same-owner refinements, and the
unchanged repeated-builder-failure regression passes on rerun. See
`final-green.log` and `regression-diagnostic-final.log`.
