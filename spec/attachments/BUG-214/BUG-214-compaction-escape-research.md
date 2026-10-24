# BUG-214: Escape does not cancel every compaction builder

## Scope and confidence

- Investigation date: 2026-10-21.
- Source reviewed: `d70f583179894f60a4f68937139f09f7d0fedb39`.
- User screenshot:
  `/Users/rquast/Desktop/Screenshot 2026-10-21 at 11.21.31 am.png`.
- The screenshot advertises `Compacting... (Esc to stop)` and displays
  `Compaction builder working 0/153`. The user reports Escape does nothing.
- This investigation traces current source and existing tests; it does not claim
  a new live-provider reproduction or identify the screenshot's exact trigger.
  Some lifecycle fixes landed after the screenshot. No production code changed
  and no runtime tests were executed as part of this research.
- Conclusion: a concrete cancellation-propagation gap exists in automatic
  pre-prompt and hook/in-loop builder execution. Other routes already have
  cancellation guards. This is not simply the historical TypeScript Esc bug.

## Keyboard and transport are already connected

1. `rust/fspec-tui/src/views/agent/input_transition.rs:173` paints
   `Compacting... (Esc to stop)`.
2. After transient UI handling,
   `rust/fspec-tui/src/views/agent/dispatch.rs:186` emits AgentEscPressed.
   Scrollback text selection, turn selection and composer selection can consume
   Escape first (`:171`); dialogs and slash popup have their own priority.
3. `rust/fspec-tui/src/app/dispatch_esc_cascade.rs:47` explicitly treats both
   Running and Compacting as active and spawns `backend.interrupt(session)`.
   It does not have the old loading-only condition from CMPCT-014.
4. Both transports forward interrupt: embedded at
   `rust/fspec-tui/src/transport/embedded.rs:98`, WebSocket at
   `rust/fspec-tui/src/transport/websocket.rs:250`.
5. `rust/rpc/src/lib.rs:1165` delegates to the manager handle;
   `rust/sessions/src/handle_impl.rs:277` resolves the session and calls interrupt.
6. `rust/sessions/src/background_session.rs:1692` sets is_interrupted, requests
   Bash abortion for the parent session ID, and notifies interrupt_notify.
   This operation does not need the long-held inner session mutex.

Thus an accepted keypress can successfully set the parent's cancellation flag
without stopping the awaited compaction builder. Repeated Escape does not add
the missing observer. The TUI also discards backend interrupt errors in its
spawned task; transport failure is another diagnostic blind spot, but is not
necessary to explain the source-confirmed builder gap.

## Where the signal is lost

### Direct automatic builder paths

- Pre-prompt threshold compaction directly awaits run_compactor_sub_agent_round
  at `rust/cli/src/interactive/stream_loop.rs:327`, before entering the normal
  stream-event loop.
- The in-stream restart/recovery macro does the same at
  `rust/cli/src/interactive/stream_loop.rs:676`.
- `rust/cli/src/compactor_sub_agent.rs:86` accepts a compaction busy flag, but
  neither a cancellation flag nor interrupt notification. It awaits the builder
  at `:130` and proceeds to commit at `:145` without a cancellation check.
- CompactionOperation uses a wall-clock timeout at
  `rust/cli/src/compaction_operation.rs:75`; a timeout is not user cancellation.
- `rust/agent-loop/src/generate_compaction_handler.rs:75` registers the normal
  builder by directly calling execute_captured_compaction_subagent. It does not
  use this module's interrupt-aware Builder::run wrapper.
- `rust/agent-loop/src/compaction_builder.rs:56` receives an explicitly unused
  `_compaction_in_progress` flag. That flag means activity, not cancellation;
  it cannot substitute for the parent interruption signal.
- `rust/agent-loop/src/compaction_execution.rs:21` awaits streaming inference;
  `:43` awaits nonstreaming multi-turn inference. Tool-free synthesis at `:97`
  likewise has no parent cancellation input. These awaits rely on their caller
  to cancel/drop them; the direct automatic routes lack that race.

The regular stream loop checks cancellation at
`rust/cli/src/interactive/stream_loop.rs:763` and listens for notification at
`:846`. Those checks cannot run while the same task is awaiting the builder.
The builder can therefore finish, validate and attempt durable promotion before
control returns to the normal interruption check. This is a source-confirmed
missing guard, not a claim that an observed user session was actually overwritten.

### Route matrix: preserve safeguards that already exist

| Route | Current cancellation behavior |
| --- | --- |
| Pre-prompt threshold builder | Direct await; no parent cancellation observer |
| Hook/in-loop recovery builder | Direct await in restart macro; same gap |
| Terminal-overflow recovery | Biased interruption race at `rust/agent-loop/src/terminal_overflow_recovery.rs:137` |
| Fresh user-input retry after failed builder | Biased interruption race at `rust/cli/src/compactor_sub_agent.rs:209` |
| GenerateCompaction self/cross-target | Builder::run races caller and target at `rust/agent-loop/src/generate_compaction_handler.rs:56` |
| Deferred TurnCompaction apply | Rejects an interrupted target before installation at `rust/agent-loop/src/turn_compaction.rs:100` |
| Manual /compact | Prepares context then queues Continue at `rust/sessions/src/handle_impl.rs:489` and `:516`; audit the eventual execution and handoff |

The screenshot phase is shared by several routes. It does not distinguish a
terminal-overflow recovery from an unguarded pre-prompt/hook path, so the exact
incident route remains unproven without correlated runtime events.

## Ownership, locking and races to address

- The agent loop holds inner session state across provider execution, starting
  at `rust/agent-loop/src/agent_loop.rs:554`. A stuck builder can delay other
  state operations, but interrupt delivery itself does not acquire that lock.
- The builder has an ephemeral tool/session identity
  (`rust/agent-loop/src/compaction_builder.rs:67`); interrupt requests Bash abort
  for the parent ID. Audit cancellation of builder-owned tools/subprocesses as
  well as the inference future. Dropping a future alone is not proof that all
  spawned work or remote inference has stopped.
- BusyOwnership in the round clears only the busy flag; background wrappers
  provide additional progress/status cleanup. Cancellation must have one clear
  owner so direct-await routes do not leave a stale Compacting spinner/footer.
- Define the cancellation-versus-promotion linearization point. Cancellation
  accepted before promotion must veto it; a generation already durably promoted
  cannot be falsely rolled back or reported as never committed. Reconcile/publish
  that authoritative boundary without resuming the original task.
- `send_input` resets interruption when enqueueing input
  (`rust/sessions/src/background_session.rs:1671`). Manual preparation followed
  by queued Continue therefore needs a handoff race test; this is a timing risk,
  not a reproduced separate defect.
- Preserve fail-closed inference readiness and cancellation as a non-retryable
  outcome. Never disguise it as a retryable builder failure and automatically
  restart it on another control-flow branch.

## Existing specifications and coverage gaps

- Historical CMPCT-014: `spec/features/esc-interrupt-during-compaction.feature`.
  Its coverage sidecar maps TypeScript AgentView.tsx/tests, not the current Rust
  builder. It establishes user intent but does not cover this runtime path.
- `rust/fspec-tui/tests/keyboard_cascade_rpc051.rs:270` checks that Escape on a
  Compacting session calls a mocked backend once. It cannot prove cancellation
  reaches or terminates the real builder.
- `rust/agent-loop/tests/bug210_automatic_continuation.rs:170` interrupts a
  pending automatic recovery and checks no restart/completion. Its terminal
  recovery guard is useful existing coverage, not proof about direct-await paths.
- `rust/agent-loop/tests/cmpct053_terminal_lock.rs:106` tests interruption of
  terminal recovery. `cmpct056_caller_interruption.rs:9` tests cross-target
  caller interruption. Keep these protected routes working.
- `rust/agent-loop/tests/cmpct056_pending_identity.rs:60` covers interruption
  after submission but before deferred apply. Extend this invariant to direct
  automatic round promotion.
- `rust/cli/tests/stream_recovery_compaction_builder.rs:77` uses immediately
  returning builders; it does not exercise Escape while those builders remain
  pending. `rust/agent-loop/tests/bug211_explicit_retry_safety.rs:11` does cover
  explicit-retry cancellation, another protected route to retain.
- `rust/fspec-tui/tests/automatic_compaction_progress_bug210.rs:107` uses task
  abort for its cancelled-cleanup case; abort is not an Escape/interrupt test.
- Related: BUG-211 explicit-retry safety, BUG-210 continuation/lifecycle and
  BUG-213 truthful status. Do not reopen the historical card as if its original
  loading-only TypeScript fix had never existed.

## Recommended fix and proposed acceptance criteria

1. Thread one operation-scoped cancellation contract through every long-running
   compaction route, including exploration, candidate corrections and synthesis.
   Use a level-triggered cancellation state plus wakeup; avoid missed wakeups or
   resetting a live operation's cancellation when queueing internal Continue.
2. Ensure Escape with no higher-priority transient UI promptly terminates pending
   pre-prompt and hook-recovery builders through the real interrupt RPC. Do not
   require a provider response or the wall-clock deadline to expire.
3. For cancellation before promotion, preserve the previous manifest generation,
   DAG, retained original identities, provider checkpoint and token snapshot.
   Emit no successful CompactionComplete and start no continuation inference.
4. Clear activity/progress/busy ownership for the original operation; report an
   explicit cancelled outcome and keep the original conversation accessible.
   Late builder output must not repaint status or commit stale candidates.
5. Exercise a barrier immediately before commit and define the post-promotion
   case explicitly; never discard an authoritative committed generation.
6. Cover streaming/nonstreaming providers and synthesis with deterministic local
   fixtures, plus both embedded and WebSocket transports. No live credentials or
   forceful task abortion should be needed to make the cancellation tests pass.
7. Preserve modal/popup/text-selection/turn-selection priority and idle Escape
   behavior. Cancellation must affect the intended session, not a newly focused
   session or another agent's operation.
8. Retain tests for terminal recovery, fresh retry, self/cross-target generation
   and manual /compact handoff so fixes do not regress already-cancellable paths.

Implement through ACDD: new/updated feature scenarios, failing route-level and
key-to-runtime tests, then minimal shared cancellation implementation. Reuse
`rust/test-helpers/` and existing provider fixtures. Scope Cargo commands to
`codelet-cli`, `codelet-agent-loop`, `codelet-sessions` or `codelet-fspec-tui` with
specific integration targets; do not run unscoped workspace tests.

## Implementation and ACDD verification

Specifications and behavioral red tests preceded each production change. The
initial automatic-route suite failed four of five scenarios; actual embedded
and WebSocket Escape tests both failed while awaiting the pre-prompt builder.
Additional red gates reproduced selected-admission cancellation in explicit
retry, terminal recovery, deferred application and cross-target generation,
as well as manual preparation/queued handoff and a surviving real subprocess.

The shared cancellation contract now observes the parent interruption flag and
wakeup while awaiting automatic builders. Selected request admission checks
cancellation before and after serialization. Retry and terminal recovery use
the same guarded round, while deferred application also observes its target
and, for cross-target generation, its caller. Cancellation is non-retryable and
does not emit success or resume the original task.

Manual continuation carries an interruption epoch rather than masquerading as
fresh user input. Cancelling preparation or a queued handoff discards the
temporary working view and restores its prior inference-safety state without
changing durable originals. Ephemeral builders own a subprocess lifetime scope;
cancellation kills their actual processes, including commands already yielded
to unified execution, without terminating another owner's work.

Review found an additional post-promotion caller bug: deferred application had
reconciled its committed generation, but agent-loop error cleanup blocked
inference again. A real inject-summary/agent-loop regression first reproduced
that failure by interrupting synchronously on committed-context publication.
Cleanup now preserves reconciled readiness, suppresses success/retry and clears
activity instead of treating accepted cancellation as exhausted compaction.

Seven focused features cover 15 scenarios, each feature mapped to one test
file. Verification uses real temporary stores and deterministic local providers,
not live-provider credentials. Escape priority, terminal/retry safety, selected
provider continuation and unified-execution regressions are retained. Legacy
provider-ambiguous fixtures, the RPC-418 fixture returning an empty session ID,
and pre-existing global tag-registry debt are tracked separately; they are not
evidence that all repository tests pass.

The final combined run passes all 22 new BUG-213/BUG-214 scenario tests. Checks
and library Clippy pass for providers, CLI, sessions, agent-loop, tools, TUI and
RPC types. The existing TUI token-state Clippy warning remains. Modified Rust
files pass formatting and whitespace checks; scoped feature syntax, registered
tags and scenario/test/implementation coverage audits pass. These are scoped
verification results, not a claim of a workspace-wide test run.
