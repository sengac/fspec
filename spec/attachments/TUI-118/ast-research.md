# TUI-118 AST research

## fspec astgrep --pattern 'Self::is_cursor_visible_for($$$A)' --lang rust --path rust/fspec-tui/src/views/agent
rust/fspec-tui/src/views/agent/hitl_keys.rs:80:20:Self::is_cursor_visible_for(session_status, &transition_state)
rust/fspec-tui/src/views/agent/hitl_keys.rs:85:9:Self::is_cursor_visible_for(session_status, &transition_state)

Definition: rust/fspec-tui/src/views/agent.rs:217 (`is_cursor_visible_for`). Busy anchor: rust/fspec-tui/src/views/agent/busy_composer.rs (`paint_busy_input_area` sets `last_hitl_input_offset`), consumed by `cursor_anchor::anchored_input_area`.
