//! RPC-095/073/402/403 — gated key-event routing for
//! [`super::multiline_input::MultiLineInput`].
//!
//! Extracted from `multiline_input.rs` so the composer file stays under
//! the 300-LoC source-shape ceiling.

use crossterm::event::{KeyCode, KeyModifiers};
use tui_textarea::Input;

use super::multiline_input::{InputEventOutcome, InputGate, MultiLineInput};

impl MultiLineInput {
    /// RPC-095 — Gated variant of `handle_key`. The `gate` is
    /// computed by the AgentView orchestrator per-frame.
    pub fn handle_key_gated(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        gate: InputGate,
    ) -> InputEventOutcome {
        // RPC-402: Enter dispositions (plain submit, Shift/Alt newline)
        // live in `multiline_input_enter.rs` to keep this file <300 LoC.
        if let Some(outcome) = super::multiline_input_enter::handle_enter(self, code, mods, gate) {
            return outcome;
        }
        // Shift+arrow chords → forwarded to caller (history / session
        // navigation). Reported as Ignored so AgentView can convert
        // them into Action::{HistoryPrev,HistoryNext,SessionPrev,SessionNext}.
        if mods.contains(KeyModifiers::SHIFT)
            && matches!(
                code,
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
            )
        {
            return InputEventOutcome::Ignored;
        }
        // Up/Down on the visual boundary of the textarea → Ignored so
        // AgentView can layer scrollback nav on top. Uses cached
        // body_width from `sync_viewport` for visual-row geometry;
        // falls back to logical-line check when width is unknown.
        if matches!(code, KeyCode::Up | KeyCode::Down) {
            let at_boundary = if let Some(body_width) = self.last_body_width {
                let (vrow, _) = self.cursor_visual(body_width);
                let total = super::multiline_wrap::total_visual_rows(self.lines(), body_width);
                match code {
                    KeyCode::Up => vrow == 0,
                    KeyCode::Down => vrow + 1 >= total,
                    _ => false,
                }
            } else {
                // Fallback: logical-line check (same as before)
                let (row, _col) = self.cursor();
                let line_count = self.line_count();
                match code {
                    KeyCode::Up => row == 0,
                    KeyCode::Down => row + 1 >= line_count,
                    _ => false,
                }
            };
            if at_boundary {
                return InputEventOutcome::Ignored;
            }
        }
        // RPC-073: Ctrl+D bubbles up so App::handle_app_shortcut
        // (Stage 4) fires the global quit fallback.
        if mods.contains(KeyModifiers::CONTROL)
            && matches!(code, KeyCode::Char('d') | KeyCode::Char('D'))
        {
            return InputEventOutcome::Ignored;
        }
        // RPC-095 — block edits gate. Swallow Backspace, Delete,
        // forward-delete, and printable character insertion
        // (`is_edit_keystroke` lives in `multiline_input_enter.rs`).
        if gate.block_edits && super::multiline_input_enter::is_edit_keystroke(code, mods) {
            return InputEventOutcome::Continued;
        }
        // COPY-007 rule [5]: an editing keystroke clears any live
        // composer selection before the edit is applied.
        if super::multiline_input_enter::is_edit_keystroke(code, mods) {
            self.selection = None;
        }
        // Everything else → forward to the textarea.
        let input = Input::from(crossterm::event::KeyEvent::new(code, mods));
        let _ = self.textarea.input(input);
        InputEventOutcome::Continued
    }

}
