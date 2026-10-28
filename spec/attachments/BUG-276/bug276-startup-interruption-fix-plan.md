# BUG-276: Preserve cancellation through prompt admission

## Outcome

Fix the lost-interrupt race. An input cancelled during queueing, hooks, persistence
or other startup work must not reach inference afterward. A genuinely new user
input can start normally without inheriting stale cancellation.

## Evidence

The isolated debug daemon accepted input at 09:43:02 UTC on 2026-10-27.
Interrupt returned in 15.2 ms at 09:43:26. Samples before and after it showed:

```text
agent_loop::prompt::admit_prompt
  user_persistence::persist_user_input
    persist::persist_user_content
      MessageStore::get_recovered_locked
        index::scan_jsonl_range -> framing::scan::scan_range
```

Startup advanced at 09:46:19, about 197 seconds after input, and the model then
generated text and called SessionSearch despite the acknowledged interruption.
The cold copied store had no valid verification certificate, intentionally
triggering recovery. This does not prove the live process scans on every turn.

`rust/agent-loop/src/agent_loop/prompt.rs` calls `reset_interrupt()` after
synchronous persistence. `BackgroundSession::send_input` also clears the flag
before queueing. The post-persistence reset exists in baseline `38a95522`; slower
persistence makes this preexisting race much easier to hit.

Local raw evidence is in `/tmp/fspec-esc-evidence/`: `startup.sample.txt`,
`after-interrupt.sample.txt`, `current.sample.txt`, `interrupt-cold.json` and
`daemon.log`. The attached shared report retains the important observations.

## Acceptance criteria to specify

1. Pause admission after an input is accepted, interrupt, release the pause:
   no provider request or tool dispatch occurs for the cancelled input.
2. Repeat with interruption during hooks and before/after persistence boundaries;
   cancellation cannot be cleared by a later startup step for the same input.
3. A subsequent explicit user input runs once without reviving the cancelled one.
4. Queued inputs, supervisor messages, automatic retries and manual compaction
   preserve their intended cancellation ownership; no blanket reset bypasses it.
5. Original input and truthful interruption outcome remain recoverable when
   persistence succeeds or fails; no false success is published.

## ACDD implementation game plan

1. Review `PromptInput`, `send_input`, `interrupt_epoch`, `admit_prompt` and manual
   compaction epochs. Define which input owns a cancellation epoch before changing
   reset ordering. Specify scenarios and validate the feature file.
2. Write deterministic integration tests with shared temp-directory helpers and
   actual admission/persistence. Use a barrier or existing storage failpoint to
   hold the boundary. A tiny fixture must reproduce the race; do not depend on
   a 16 GB scan or long sleeps. Record RED, including zero provider dispatch.
3. Capture/reset cancellation ownership at the deliberate new-input boundary,
   propagate it through startup and recheck it before expensive setup/inference.
   Reuse existing epochs where appropriate rather than introducing a competing
   cancellation system. Do not simply move one reset while leaving queued-input
   races untested.
4. Move long synchronous startup work off async workers where necessary without
   dropping an in-progress durable transaction. Cancellation acknowledgement
   and safe settlement follow BUG-275's contract.
5. Verify cold resume, repeated Escape and new-input recovery; repeat the isolated
   admission probe and demonstrate that the provider is not contacted after cancel.

## Validation targets

Add a dedicated `bug276_prompt_admission_cancellation` integration target, plus:

```sh
cargo test -p codelet-agent-loop --test rpc088_interrupt_cascade
cargo test -p codelet-agent-loop --test bug246_recovery_admission
cargo test -p codelet-agent-loop --test bug246_recovery_boundaries
cargo test -p codelet-agent-loop --test bug220_interruption_provenance
cargo test -p codelet-cli --test bug214_compaction_cancellation
```

Existing source-shape assertions may encode the faulty reset location; replace
them with meaningful behavior coverage, not weaker checks. Run scoped check,
clippy, formatting, feature/tag validation and scenario coverage before closure.
Never mutate or profile the user's live process/history.
