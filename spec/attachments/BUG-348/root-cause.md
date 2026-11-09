# BUG-348 root cause (from TUI-115 research)

`BackgroundSession::submit_input` (rust/sessions/src/background_session/input_modes.rs) echoes a steer at once in the `(true, InputMode::Steer)` arm (`handle_output(StreamChunk::user_input(..))`) and parks it with `input_inbox::push_steer`.
If the running turn ends before it reaches a safe boundary that drains the steer, `continue_with_inbox` (input_modes.rs:75) takes the parked steers, joins their texts with "\n\n" and calls `send_input`, which echoes `StreamChunk::user_input(joined)` again.
So each such steer appears twice in the transcript: once as `You (steer): a` (TUI-115 tag) and again as `You: a`.
Fix direction: the second send should not re-echo text that was already echoed (e.g. a send path without the echo, or an echo flag on the inbox entry).

## Fix (2026-11-02)
- `BackgroundSession::send_input` now delegates to a private `start_turn(input, thinking_config, TurnEcho)`. `continue_with_inbox` calls the new `send_steers_as_turn`, which uses `TurnEcho::AlreadyEchoed`: the submission is still retained and sent, but `StreamChunk::user_input(joined)` is not echoed a second time.
- Red (run 4): rust/agent-loop/tests/bug348_steer_echoed_once.rs failed at :57, with 2 echoes of "use the staging database" against an expected 1.
- Green (run 5): bug348 1/1 and the rig016_interrupt_steer_queue regression 5/5. `clippy -p codelet-sessions --lib -D warnings` is clean.
