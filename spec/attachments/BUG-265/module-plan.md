# BUG-265 approved mechanical split plan and invariant ledger

The actual request_user_input response selected **Approve Split**. The supervising
caller expressly acted under the user's in-scope reasonable-approval grant, not
as fabricated new human speech. The native discovery question is answered with
that attribution and the exact bounds. No production move has happened yet.

## Named module plan

- `background_session.rs`: keep the BackgroundSession declaration, existing field
  visibility, module wiring and public re-exports; keep this file below 300 lines.
- `background_session/types.rs`: input, incoming message/image, work-unit context,
  progress and checkpoint error types; preserve their original public paths by
  re-exporting them from the parent.
- `construction.rs`: existing constructor and channel/default initialization.
- `workspace.rs`: owner, effective working directory, isolation and checkpoints.
- `focus.rs`: work-unit context and authorization/reminder state.
- `metrics.rs`: tokens, model IDs/limits and thinking-level getters/setters.
- `output.rs`: status, callbacks, output publication and correlation bookkeeping.
- `transcript.rs`: archival seed/retrieval, subscriptions and restore markers.
- `interactions.rs`: pause/fspec/HITL channels, snapshots and request-ID gates.
- `exec_stdin.rs`: existing stdin request overlay and process liveness checks.
- `completion_contract.rs`: continue/goal chrome and generation-gated sync.
- `messaging.rs`: role/input/incoming messaging and loop-abort notes.
- `history.rs`: clear history, completed-turn count and session info.
- `progress.rs`: compaction progress and pending correlation IDs.

Reuse `interruption.rs`, `compaction_handoff.rs` and `agent_message.rs` as-is.
Keep every new/moved source and test module below 300 lines; subdivide a cohesive
group if needed. Existing source-shape tests are large too: split their scenario
functions into bounded child modules, use shared source-reading helpers and keep
every original condition. Read moved implementations rather than weakening tests
to names or deleting assertions. Any split-induced failure is introduced, not
preexisting. No alternate business-logic helper may conceal the original defect.

## Invariants

1. BackgroundSession fields remain in the parent with identical visibility/types.
2. Public type/method paths, re-exports and constructor parameter order stay stable.
3. Constructor defaults, channel senders/receivers and ownership/drop order stay
   unchanged. No new runtime, dependency, service, thread or session engine.
4. Atomics, Acquire/Release/AcqRel/SeqCst orderings and all generation checks remain
   verbatim during the mechanical phase.
5. Admission/focus authorization, request-ID checked responses, pause/HITL/stdin,
   interrupt/cancellation and history-cleared publication retain their behavior.
6. Private associated helpers moved to child modules may use `pub(super)` solely
   to restore their old parent-module visibility to sibling implementations; no
   externally widened visibility is authorized.
7. Existing root lint attributes are preexisting, not permission to add new lint
   suppressions. New code must be fallible and avoid unwrap/panic/unsafe.
8. Capture scoped runtime controls before moves and after mechanical extraction;
   capture genuine BUG-265 reds before any budget/goal behavior change.
9. The behavior repair distinguishes real user input from automatic compaction,
   loop-abort and other synthetic dispatch. Unchanged goals preserve rejection
   counts; genuine task starts reset nudges; actual goal changes/clear reset goal
   accounting. CONT-008 generations and CONT-009 arming order stay authoritative.

## Validation bounds

Use offline ci-test package/target-scoped jobs=2 and inspect disk before heavy
compilation; stop at the strict 6 GiB free floor. Cover CONT-009 sync, CONT-008
generation/replacement races, background output/channels, admission, checked
interaction responses and real shared-loop recovery accounting. Existing
background_session_shape includes nested default-profile Cargo builds: do not run
those unsafe subprocess scenarios as a broad target under these constraints.
Read and select bounded nonbuilding scenarios or adapt their command profile only
with explicit relevant approval; never claim unrun scenarios passed.

One read-only discovery shell command was blocked by the cargo-output-pipeline
guard because its search pattern contained Cargo command text and a separate
file-list pipeline. It did not execute; it was not retried or bypassed. No test
or code result is inferred from that blocked read.
