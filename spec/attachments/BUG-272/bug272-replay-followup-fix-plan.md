# BUG-272: Fix terminal-error history deletion — additional incident

## Scope and priority

Complete the existing BUG-272 implementation; do not create a duplicate card or
leave it as research-only intake. Its current testing status, existing feature,
rules and tests remain authoritative. This plan adds the new concrete incident
and validation steps. No production fix is claimed here.

## Exact incident evidence

The user supplied `Screenshot 2026-10-27 at 7.25.03 pm.png`. Its relevant sequence:

```text
API Error: HttpError: Http client error: Codex network setup failed
[Removed the failed tool call from context; send your next message to continue]
Next user input: ...
API Error: RequestError: Codex replay has no recorded outcome for call
call_c324aa98a5e44ca39cfdb63dc71a0608
```

The original persisted envelopes were retrieved with `SessionSearch originals`
for session `d56fde15-6c2d-463d-aefd-7ec6e1fdbac5`:

| Position | Evidence |
| --- | --- |
| 554 | Assistant shell_command invocation with call_id `call_c324aa98a5e44ca39cfdb63dc71a0608`, tool_use ID `fc_082b4fd8e09e7d74016ae06ca6ca748191b31ace799802b9df` |
| 555 | Matching tool_result with the same IDs, `is_error: false`, and shell output containing `success: true` |
| 556 | Next human input `...` |

The tool had completed and its genuine result existed on disk. The error is
about assembled provider replay, not proof that the tool never produced a result.
Do not rerun completed commands as a recovery strategy: other tools have effects.

## Causal source path

1. `rust/cli/src/interactive/stream_loop.rs:2345` invokes generic terminal-error
   `strip_failed_tool_call_tail` when not interrupted, including setup failures.
2. `recovery_unrecoverable.rs` deletes the result and matching call from the
   active `session.messages`, without reconciling the Codex checkpoint.
3. `rust/providers/src/codex/history_lineage.rs` retains committed call ancestry
   when assembling the next provider request.
4. `history_replay.rs:20` rejects an assembled call without an output.

The existing feature already requires preserving these outcomes. Keep its
missing-outcome refusal: weakening validation would hide damage, not repair it.

## ACDD implementation game plan

1. Review `codex-terminal-error-history-preservation.feature` and its coverage.
   Add a network/setup-failure example to the rejection scenario or a dedicated
   scenario if it exercises a different path; keep 1:1 scenario/test mapping.
2. Run the existing `bug272_terminal_error_history` target to establish current
   RED evidence. Add a real scripted-transport test: execute a filesystem-changing
   tool once, fail the following provider request during setup, then retry with
   a new input. Assert the exact original IDs/outcome remain and side-effect count
   stays one. Include cold resume and genuine error outcomes.
3. Exempt Codex-owned ancestry from generic terminal-error tail deletion. Preserve
   the real call/result and surface the typed provider failure. Keep intended
   non-Codex behavior and earlier context-overflow recovery unchanged.
4. Verify a truly unresolved checkpoint still fails safely before network dispatch;
   do not synthesize success/cancellation, reset checkpoints, rotate affinity or
   silently repair history to make the test pass.
5. If repairing already-affected sessions is necessary, design an explicit,
   identity-checked reconciliation from genuine durable outcomes. Cover it with
   a separate scenario before coding; do not modify this user's live checkpoint
   or invent outputs during card implementation.
6. Run scoped regressions and attach RED/GREEN logs. Confirm continuation and cold
   resume replay the original result once and no removal notice is shown.

## Validation targets

From `rust/`:

```sh
cargo test -p codelet-cli --test bug272_terminal_error_history
cargo test -p codelet-providers --test codex_replay_regressions
cargo test -p codelet-providers --test cmpct080_checkpoint_replay
cargo test -p codelet-cli --test codex_interruption_call_pairs
```

Run scoped check/clippy, formatting, feature/tag validation and coverage checks.
Coordinate with BUG-275's cancellation settlement and preserve truthful original
outcomes in both paths. Keep BUG-272 in its existing workflow state until actual
RED/GREEN implementation evidence justifies advancement.
