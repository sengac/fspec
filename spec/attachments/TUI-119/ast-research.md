# TUI-119 AST research

## grep findings
- `rust/fspec-tui/src/store/agent_view/session_context.rs:138` `SessionContext::reset_scrollback` (no busy_inputs reset).
- `rust/fspec-tui/src/store/agent_view/record_chunk.rs` handles `StreamChunk::Interrupted { .. }` without reading `queued_inputs` (grep `queued_inputs` in rust/fspec-tui/src finds nothing).
- `rust/sessions/src/background_session/input_modes.rs:55` `queued_inputs()` is the session-side source (oldest first).

## fspec astgrep --pattern 'transcript_push_action' --lang rust --path rust/fspec-tui/src/app
rust/fspec-tui/src/app/bootstrap.rs:170:74:transcript_push_action
rust/fspec-tui/src/app/dispatch_transcript_live.rs:37:15:transcript_push_action

Every live chunk goes through `transcript_push_action`; `is_transcript_content` sends Interrupted/InterruptedWithReason to `TranscriptAction::Live`, so `handle_chunk_received`/`record_chunk` only see them again from transcript pages.

## grep 'queued_inputs: Vec::new()' rust/sessions/src/replay_transcript.rs
The journal replay builds `InterruptedWithReason { queued_inputs: Vec::new(), .. }` for UserCancelled/LoopInterrupted terminals.
