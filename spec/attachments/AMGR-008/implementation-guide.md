# AMGR-008 — Remove Old Supervisor Infrastructure

## Summary

Cleanup story. Remove all the old supervisor observation/injection machinery that is being replaced by the explicit pull (SessionSearch) and push (message) model. No new features — just removal and verification that nothing breaks.

## What to Remove

### In `codelet/napi/src/session_manager.rs`:
- `supervisor_agent_loop` async function — the specialized loop with biased tokio::select! that subscribed to broadcast, accumulated observations, detected breakpoints, and injected
- `ObservationBuffer` struct and impl — accumulated stream chunks for supervisor evaluation
- `format_evaluation_prompt` function — formatted accumulated observations for LLM evaluation
- `evaluate_and_maybe_inject` function — the LLM call to decide whether to inject
- `SupervisorRole` struct — replaced by simple role string on any session
- `SupervisorInput` struct — the injection payload (session_id, role_name, message, images)
- `format_supervisor_input` function — formatted injection text
- `create_supervisor_session_with_id` method — created sessions with supervisor_agent_loop instead of regular agent_loop
- `session_create_supervisor` NAPI function — the TypeScript-facing supervisor creation
- `supervisor_inject` function — the injection entry point
- `receive_supervisor_input` method on BackgroundSession — queued SupervisorInput for processing
- `supervisor_input_sender` method — exposed the mpsc sender
- `supervisor_input_tx` / `supervisor_input_rx` fields on BackgroundSession — the injection channel
- Broadcast subscription logic in supervisor_agent_loop

### In TUI code:
- `/supervisor` command handler
- `SupervisorTemplateList` view/component
- `SupervisorCreateView` view/component  
- `SupervisorTemplateForm` view/component
- Any supervisor template storage/persistence

### In test files:
- Tests for supervisor_agent_loop
- Tests for ObservationBuffer
- Tests for SupervisorInput
- Tests for breakpoint detection
- Tests for format_evaluation_prompt

## What to Keep (Modified)

- **ChainOfCommand** — keep the data structure but simplify. It now only tracks spawner→spawned ownership for:
  - close permission checks (only spawner can close)
  - list/get_status relationship reporting
  - No observation streaming through it
- **broadcast::channel on BackgroundSession** — may still be useful for TUI display of session output, but supervisors no longer subscribe to it for observation
- **BackgroundSession** — stays, but remove supervisor-specific fields (supervisor_input_tx/rx, etc.)
- **SessionManager** — stays, remove supervisor-specific methods

## Validation

1. `cargo build` succeeds with no errors
2. `cargo test` passes — all remaining tests pass
3. Existing sessions work normally as regular agent_loop sessions
4. TUI launches without /supervisor command
5. No references to removed types/functions remain (except in git history)

## Risk

Low — this is removal, not modification of working features. The old supervisor pipeline was a separate code path. Removing it doesn't affect the regular agent_loop path that all sessions will now use.
