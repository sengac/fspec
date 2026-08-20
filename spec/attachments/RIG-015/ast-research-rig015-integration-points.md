# AST Research — RIG-015 integration points (discovery)

Indexed the full `rust/` workspace into the knowledge graph (80,433 entities).
Key entities confirmed for the RIG-015 behavioral test:

## 1. `feed_loop_detectors` (agent-loop/src/background_output.rs:114-199)
- `fn feed_loop_detectors(&self, channel: &str, delta: &str) -> bool`
- On `LoopEscalationOutcome::Abort`:
  - sets `loop_abort_fired`
  - appends `build_loop_abort_marker_note()` to assistant content
  - `self.session.set_pending_loop_abort_note(build_loop_abort_recovery_message(&signal, None))`
  - `self.session.interrupt()` (drives existing interrupt machinery)
  - returns `true`
- Called from `BackgroundOutput::emit` for `StreamEvent::Text` ("text") and
  `StreamEvent::Thinking` ("thinking").

## 2. `set_pending_loop_abort_note` (sessions/src/background_session.rs:1518-1522)
- `pub fn set_pending_loop_abort_note(&self, note: String)` — stores into
  `pending_loop_abort_note: std::sync::Mutex<Option<String>>`.
- Consumed by `take_pending_loop_abort_note` (same file) at the top of the
  NEXT turn in `agent_loop.rs` (~line 517), injected as a `Message::User`
  into `inner_session.messages`.

## 3. `run_agent_stream_with_images` (cli/src/interactive/stream_loop.rs)
- 9 positional args: `agent, input, images, session(&mut), is_interrupted,
  compaction_in_progress, interrupt_notify, output(&O), session_id`.
- The stream loop checks `is_interrupted.load(Acquire)` at the top of each
  iteration (line 734) and on the NAPI-mode `interrupt_notify.notified()`
  arm (line 815) — both break out and call `output.emit_interrupted` +
  `output.emit_done_with_stop_reason`.
- Text deltas flow through `handle_text_chunk` → `output.emit_text` →
  `BackgroundOutput::emit(StreamEvent::Text)` → `feed_loop_detectors`.

## 4. Stub provider seam (test-support)
- `codelet_providers::stub_model::StubModel` implements
  `rig::completion::CompletionModel`; `stream()` currently yields a canned
  `RawStreamingChoice::Message("hi back")` + `FinalResponse`.
- `agent_loop.rs` "stub" arm (line 1139, `#[cfg(feature = "test-support")]`)
  constructs `StubProvider::new().create_rig_agent(...)` and drives
  `run_agent_stream_with_images` — the ONLY production path that constructs
  the real `BackgroundOutput` and runs the RIG-014 detector wiring.
- `rpc072_work_agent_roundtrip.rs` shows the full end-to-end harness:
  `SessionManager::new()` + `set_hooks(FspecAgentHooks)` +
  `register_stub_provider()` + `set_default_model("stub/canned")` +
  `create_session_with_id` + `send_input`, then asserts on the
  `chunks_tx` broadcast.

## Design conclusion
The test drives the FULL production `agent_loop` via the stub provider arm.
To make the stub emit a looping stream, `StubModel` gains a test-only
configurable stream source (behind `test-support`): a static
`OnceLock<Option<(Vec<String>, Arc<AtomicUsize>)>>` that, when set, makes
`stream()` yield each word as a `RawStreamingChoice::Message` delta
(incrementing the shared poll counter) followed by `FinalResponse`.
The test asserts:
1. poll counter << full stream length (stream actually cancelled mid-stream)
2. `session.is_interrupted` is true
3. persisted assistant message ends with the marker note, no degenerate tail
4. next turn's chat history contains the corrective note
