# TUI-115 AST research (started by the MCP-004..006 agent, 2026-11-02)

## fspec astgrep --pattern 'paint_input_or_spinner_with_waiting($$$A)' --lang rust --path rust/fspec-tui/src/views/agent
rust/fspec-tui/src/views/agent/input_area.rs:177:9:paint_input_or_spinner_with_waiting(padded, buf, &self.input, &state, waiting)
rust/fspec-tui/src/views/agent/input_area.rs:221:13:paint_input_or_spinner_with_waiting(padded, buf, &self.input, &state, waiting)


## fspec astgrep --pattern 'Action::InputQueued($A)' --lang rust --path rust/fspec-tui/src
rust/fspec-tui/src/views/agent/queue_key.rs:22:19:Action::InputQueued(value)


## fspec astgrep --pattern 'InputMode::Steer' --lang rust --path rust/fspec-tui/src
No matches found

## fspec astgrep --pattern 'spawn_submit_input($$$A)' --lang rust --path rust/fspec-tui/src/app
No matches found

## grep findings (what already exists)

- Backend steer/queue/interrupt already exist (RIG-016): `rust/rpc-types/src/input_mode.rs` `InputMode { Steer (default), Queue, Interrupt }`;
  `rust/sessions/src/background_session/input_modes.rs` `BackgroundSession::submit_input(text, thinking, mode)`
  steers into / queues behind a Running **or Compacting** turn (`turn_running`). TUI-115 needs no backend or wire change
  for steering; it must make Compacting+Enter use `InputMode::Queue` instead of the default Steer.
- TUI submit paths: `rust/fspec-tui/src/app/dispatch_slash_submit.rs:158` submits with `InputMode::Steer`;
  `rust/fspec-tui/src/app/dispatch_input_modes.rs` (`spawn_submit_input`, `handle_input_queued`) handles
  `Action::InputQueued`, emitted by `rust/fspec-tui/src/views/agent/queue_key.rs` on Alt+Enter while
  `last_is_running` (Running only — `last_is_compacting` is tracked separately in `pane_render.rs:186`).
- The composer is hidden while busy: `rust/fspec-tui/src/views/agent/input_area.rs:177/221` call
  `input_transition::paint_input_or_spinner_with_waiting`, which paints the spinner row INSTEAD of the MultiLineInput
  when the per-session `InputTransitionState` is `Loading`/`Compacting`/`Hiding`/`Showing`
  (`rust/fspec-tui/src/views/agent/input_transition.rs`). `input_area_height` (input_area.rs:50) sizes one slot.
- No `You (steer)` / `Queued #n` rendering exists yet (grep finds none).

## Suggested implementation shape (for the next agent; not yet specified as scenarios)

1. Layout: while Running/Compacting, `input_area_height` grows to activity row + composer rows + one hint row;
   `paint_input_area` paints the spinner/transition row above the MultiLineInput instead of replacing it, then the hint
   (`Enter steer · Alt+Enter queue · Esc interrupt`, or the compacting variant where both keys queue).
   Files near 300 lines: `views/agent.rs` (294) and `views/agent/pane_render.rs` (248) — split under a REFAC card
   first if an edit would pass 300.
2. Keys: Enter while Running → existing steer submit; Enter or Alt+Enter while Compacting → `InputMode::Queue`
   (extend `queue_on_alt_enter` / the submit path with `last_is_compacting`).
3. Transcript tags: record the mode of each busy submission and render `You (steer): …` / `Queued #n: …`; queued
   entries listed (e.g. under the activity row) until the session reports them started — check which session event
   signals that a parked/queued input starts (RIG-016 inbox in `sessions/src/background_session/messaging.rs`).
4. Tests: fspec-tui uses ratatui buffer/snapshot tests (see `rust/fspec-tui/tests/`); one test file per feature.

## Handover (MCP-007 agent, 2026-11-02)

- The feature is written: `spec/features/running-composer-steer-and-queue-hints.feature` (7 scenarios, tagged `@wip @TUI-115`), with a hand-written empty `.feature.coverage` skeleton.
- The card is estimated at 8 points, has three architecture notes, and is now in **testing**. No tests or code exist yet.
- Next step: write `rust/fspec-tui/tests/tui115_running_composer_steer_queue.rs`, one test per scenario with an `@step` comment for every step, run it red, then link coverage with `fspec link-coverage running-composer-steer-and-queue-hints ...`.
- Before implementing, check whether `views/agent.rs` (294 lines) needs a REFAC split. Any edit that would take it past 300 lines needs one.
- Open point for the test author: which session event marks a queued input as started? Candidates are the RIG-016 inbox delivery in `sessions/src/background_session/messaging.rs` and the next `SessionStateChange` to Running that echoes the input. Pin this down before writing the queued-list assertion, because the scenario only states the observable result.

## Open point resolved from the code (MCP-008 agent, 2026-11-02)

The question was which session event marks a queued input as started. The code answers it; no product decision is needed.
- A queued input starts only through `BackgroundSession::resume_queued` → `send_input` (in `rust/sessions/src/background_session/input_modes.rs` and `messaging.rs:61`).
- `send_input` first emits `StreamChunk::user_input(text)` (the UserInput echo), then sets the status to Running (`SessionStateChange`).
- So the TUI drops a queued entry when it receives a UserInput chunk whose text equals the **oldest** queued entry's text. Queued inputs start oldest first, through `pop_queued`.
- Steers that a turn never reached are joined into one turn by `continue_with_inbox`, with their texts joined by `"\n\n"`. Their echo matches no queued entry, so it must not clear one.
- A `Steer` submitted to a busy turn is echoed immediately with `StreamChunk::user_input` (`input_modes.rs`, the `(true, InputMode::Steer)` arm). The TUI's `You (steer)` tag must therefore replace that echo, not appear next to it, or the text shows twice.
- `Interrupted { queued_inputs }` and `InterruptedWithReason { queued_inputs }` carry the session's authoritative queued list (oldest first). On those chunks the TUI should resync its list from that field.

State: still in **testing**, no test file yet. Next step: read the fspec-tui test harness in `rust/fspec-tui/tests/`, write `rust/fspec-tui/tests/tui115_running_composer_steer_queue.rs`, run it red, and link coverage. `views/agent.rs` is still 294 lines.

## Handover (MCP-009/MCP-010 agent, 2026-11-02)

This agent stopped before TUI-115 because the card is too large for the context it had left. Nothing in the TUI-115 code or tests has been touched. The card is still in **testing** and its feature is still `@wip`. Findings to start from:
- **Harness:** copy `rust/fspec-tui/tests/rig016_steer_queue_keys.rs`. It builds `App` + `common::MockBackend` and provides the `drain`, `key`, `type_text` and `render` (TestBackend 160x30) helpers, plus `running_agent_view()`, which creates a session with `script_create_session` and the '.' key, then calls `agent_view_store_mut().set_session_status(.., Running)`.
  - `MockBackend::submitted_inputs()` returns `(SessionId, text, InputMode)`.
  - `push_chunk(id, StreamChunk)` and `push_status_change(id, SessionStatus)` simulate the session (`tests/common/mock_scripting_1.rs:36/50/265`).
  - `SessionStatus::Compacting` exists (`rpc-types/src/sessions.rs:74`).
- **Transcript echo:** `StreamChunk::UserInput { text }` becomes a `ChunkKind::UserInput` chunk (`fspec-tui/src/store/agent_view/record_chunk.rs:48`). The renderer adds the `"You: "` prefix (`store/agent_view/chunk_wrap.rs:61`).
  - For "You (steer): " / "Queued #n: ", record the mode of each busy submission per session in the agent-view store. Then either tag the session's matching UserInput echo (steer) or push a tagged chunk (queued) and suppress the later start echo, so the text never appears twice (see "Open point resolved" above).
- **Size check:** `views/agent.rs` is still 294 lines; `pane_render.rs` 248, `input_area.rs` 245, `input_transition.rs` 263. Open a REFAC card before any edit that takes one past 300.
- **Next step:** write `rust/fspec-tui/tests/tui115_running_composer_steer_queue.rs`, one test per scenario with an `@step` comment for every step. Run it with `cargo test --profile ci-test -p codelet-fspec-tui --test tui115_running_composer_steer_queue`, confirm it is red, then `fspec link-coverage running-composer-steer-and-queue-hints ...`.

## Implemented (2026-11-02)

- Busy layout: `views/agent/busy_composer.rs` paints the activity row, then the queued list, then the composer, then the hint. `input_area.rs` routes to it whenever the focused session's status is Running or Compacting. `input_transition::paint_activity_row` paints the activity row by itself.
- Tags: the new `ChunkKind::TaggedUserInput(UserInputTag::{Steer, Queued(n)})` variant. `ChunkKind::UserInput` is unchanged, so no wire or test churn. Per-session `BusyInputs` lives in `store/agent_view/busy_inputs.rs`. A steer's echo gets the steer tag. A queued input is shown when it is submitted, and its start echo (matching the oldest queued text) removes it from the list and is not repeated.
- Keys: `App::busy_enter_mode` returns Running → Steer and Compacting → Queue. Alt+Enter queues while Running or Compacting. The RPC-095 compacting edit gate is open in the agent view, so paste and typing work during compaction. The RPC-403 and BUG-179 compacting-paste scenarios were updated to match.
- Follow-ups carded: TUI-118 (cursor while busy), TUI-119 (Interrupted resync and reset), BUG-348 (unreached steers echoed twice).
