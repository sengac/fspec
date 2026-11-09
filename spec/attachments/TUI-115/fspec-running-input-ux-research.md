# Running agent input and stdin interruption: discovery research

Status: discovery only. Discuss the open questions with the user before deciding UX or writing acceptance criteria. These observations describe current source and specifications, not a proposed design or a manually reproduced incident.

## Running chat composer and input modes

- `rust/fspec-tui/src/views/agent/input_area.rs:163` delegates the input row to the transition renderer. `rust/fspec-tui/src/views/agent/input_transition.rs:180` and `:228` show Loading/Compacting rendering a spinner instead of input; only Idle paints the composer, and `:167` limits the cursor to Idle. A running draft can therefore be editable without being visible, depending on transition state.
- `rust/fspec-tui/src/views/agent/dispatch.rs:203` routes Alt+Enter to `queue_key.rs:12`, which reads and clears the shared input and emits a queued action when the cached running flag is true. Normal submitted input is routed to steer for Running or Compacting by `rust/fspec-tui/src/app/dispatch_slash_submit.rs:153` and `dispatch_input_modes.rs:10`. However, the normal composer gate blocks edits and suppresses Enter during Compacting (`rust/fspec-tui/src/views/agent/dispatch.rs:238`); application-level mode routing alone does not mean the ordinary key path permits steering then.
- `spec/features/agent-view-steer-and-queue-keys.feature:18` specifies the bindings. `rust/fspec-tui/tests/rig016_steer_queue_keys.rs:72` tests submitted modes using a rendered running view and typed keys, but does not assert draft visibility in the rendered buffer. `spec/features/interrupt-steer-and-queue-input.feature:6` describes delivery timing and parked queues; steer is not necessarily immediate interruption of an in-flight tool.

## Automatic stdin prompt and collision

- `rust/tools/src/unified_exec/exec_stdin.rs:35` uses a 3-second quiet threshold and 30-second cooldown; quiet time does not prove a process needs input. `spec/features/exec-stdin-prompt.feature:28` explicitly acknowledges false positives for quiet non-interactive commands.
- Push events populate the prompt while the agent remains running (`rust/fspec-tui/src/app/dispatch_stream_chunks.rs:181`; `spec/features/exec-stdin-push-tui-composer.feature:23`). `rust/fspec-tui/src/views/agent/input_area.rs:101` and `:131` prioritize HITL > exec-stdin > pause > normal composer. The stdin prompt renders the same `self.input`, rather than an independently saved chat draft.
- `rust/fspec-tui/src/views/agent/dispatch.rs:112` routes HITL then stdin before normal input and queue handling. `rust/fspec-tui/src/views/agent/exec_stdin_keys.rs:34` sends plain Enter's entire shared buffer to command stdin and clears it; Alt+Enter is handled by the shared input instead of the queue handler; Esc dismisses only the overlay, while Ctrl+C interrupts the session. Consequently an arriving prompt can change the destination of an already-typed chat draft and repurpose expected shortcuts. This is an inference from code paths, not a verified interactive reproduction. Esc leaves the shared buffer untouched, but that is not isolation of separate drafts; submission clears it. On write failure the slot remains and the error is logged (`rust/fspec-tui/src/app/dispatch_exec_stdin.rs:94`), after the key handler has already cleared the input.
- Existing coverage includes mode submission (`rust/fspec-tui/tests/rig016_steer_queue_keys.rs:72`), stdin submit and Esc draft preservation (`rust/fspec-tui/tests/exec_stdin_prompt_p2.rs:359` and `:418`), and push/freshness behavior (`rust/fspec-tui/tests/exec_stdin_push_chunk_bug171.rs`, `rust/fspec-tui/tests/exec_stdin_probe_freshness_bug209.rs`). These references do not establish an end-to-end test of mid-draft prompt arrival, wrong-destination prevention, or the combined visible-running-composer interaction.

## Questions for discussion with the user

1. During Running and Compacting, should the composer be visible, appear when typing, or remain hidden? How should the activity indicator coexist with it?
2. What should Enter and Alt+Enter do, and how should steer and queue be discoverable and their delivery timing communicated?
3. Should a quiet-command stdin request automatically take focus, or be a non-interrupting choice? How should false positives and repeated prompts behave?
4. When a prompt arrives during a chat draft, should command and chat drafts be separate, and how should both be restored or protected against accidental delivery to the wrong destination?
5. While a stdin request is active, how should users switch between stdin, steer and queue, and what should Esc and Ctrl+C mean?
6. What feedback and recovery are wanted for failed writes, prompt dismissal, process exit, and focus/session changes?

After discussion, use the answers to develop examples and acceptance criteria, including prompt arrival while typing, failed write, dismissal, session switching, and queue/steer delivery. No UX decision or implementation is made by this research.

## Verification performed

Source, existing feature descriptions, and targeted test source were inspected for render, routing, detector, and failure behavior. No interactive reproduction or test suite was run for this discovery-only card; rendering and cross-feature risk remain to be validated against the desired user experience.
