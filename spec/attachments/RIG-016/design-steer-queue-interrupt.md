# Design: steer, queue and interrupt (RIG-016)

## What exists (2026-11-01)
- `BackgroundSession::send_input` (`rust/sessions/src/background_session/messaging.rs:61`) retains a durable
  submission receipt (`retain_submission`), then pushes a `PromptInput` onto `input_tx`. Input sent while a turn runs
  already waits in the channel and runs as the next turn, but nothing parks it on interrupt, and there is no in-turn
  steering.
- The CLI stream loop (`rust/cli/src/interactive/stream_loop/`) drives rig's multi-step tool loop.
  `tool_result::handle` appends each tool result to `session.messages`. The recovery path (`restart.rs`) can rebuild
  the stream mid-turn with `agent.prompt_streaming_with_history_and_hook(prompt, session.inference_messages(), hook)`.
- `interrupted.rs` publishes `StreamEvent::Interrupted(queued)`, whose list comes from an optional `InputQueue`. The
  daemon path passes `None`, so it is always empty. The `Interrupted { queued_inputs }` wire chunk already exists.

## Design
1. **Per-session inbox** (`codelet_common::input_inbox`, a process registry keyed by session id, like the other
   per-session registries). It holds a FIFO of steers and a FIFO of queued inputs. Sessions write to it, and the CLI
   stream loop and agent loop read from it, so no crate gains a new dependency.
2. **`BackgroundSession::submit_input(text, thinking, mode)`**. When idle, every mode behaves like `send_input`. When
   busy, it retains a durable submission receipt first, so accepted input is never silently dropped. Then:
   - **steer** pushes to the steer FIFO and emits a `UserInput` chunk;
   - **queue** pushes to the queued FIFO;
   - **interrupt** calls `interrupt()` and parks the text at the front of the queue.
3. **Steer at a safe boundary.** In `stream_loop::tool_result::handle`, once a step's tool calls have all settled (no
   tool execution in progress and no unresolved buffered calls), the loop drains pending steers. It then restarts
   the stream with the joined steer text as the next user message over `inference_messages()`, which already holds the
   assistant tool call and its result. This keeps tool-call history valid and never alters an in-flight generation.
4. **Queue as the next turn.** When a turn completes normally, the agent loop pops the next queued input and submits
   it as a new turn, one at a time and in submission order.
5. **Interrupt parks the queue.** `interrupted.rs` reports the queued FIFO (without draining it) as `queued_inputs`,
   and the agent loop does not pop queued inputs after an interrupted turn.
   `BackgroundSession::resume_queued()` (the explicit resume) pops the next parked input and starts it.
6. **Wire and TUI.** A `submit_input(session_id, text, mode)` RPC is appended at the end of `FspecService` (RPC-449
   rule), and `FspecBackend::submit_input` covers both transports. In the agent view, while the session is running,
   Enter submits a steer, Alt+Enter submits a queue entry, and Esc keeps interrupting. A resume control sends the
   parked inputs.
