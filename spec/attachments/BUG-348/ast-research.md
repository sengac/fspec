# BUG-348 AST research

## fspec astgrep --pattern 'input_inbox::take_steers($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/background_session/input_modes.rs:76:22:input_inbox::take_steers(self.id)

`continue_with_inbox` joins the steers with "\n\n" and calls `self.send_input(text, thinking_config)`; `send_input` (messaging.rs) emits `StreamChunk::user_input(text)` first.
