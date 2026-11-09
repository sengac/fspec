//! TUI-116 — key routing for the stdin notice and the stdin answer box.
//!
//! Feature: spec/features/stdin-notice-and-answer-box.feature
//!
//! Consulted AFTER the HITL keys (HITL still wins) and BEFORE the pause
//! keys, only while the FOCUSED session has an exec-stdin request (cached on
//! the view as `last_exec_stdin` at render time).
//!
//! Chat focused (the notice takes no focus):
//!   Ctrl+O → focus the answer box
//!   Ctrl+X → `ExecStdinMuted` (TUI-117: clear the slot and mute this
//!            command run; nothing reaches the command)
//!   any other key → `None`, so it keeps its chat meaning (Esc interrupts a
//!                   running turn, typing edits the chat draft)
//!
//! Answer box focused (every key stays in the box):
//!   Enter  → `ExecStdinSubmit` with the box line (empty lines send nothing)
//!   Esc    → back to the chat draft; the notice and the box text stay
//!   Ctrl+C → `ExecStdinControl(Interrupt)`: SIGINT to the command only
//!   Ctrl+D → `ExecStdinControl(Eof)`: end of the command's input
//!   Shift+Up / Shift+Down → walk the box history of unsent text (TUI-117)
//!   typing / paste → the box's own `MultiLineInput` (clears the error row)

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use codelet_rpc_types::{ExecStdinControl, SessionId};

use crate::components::{Action, EventResult};

use super::AgentView;

impl AgentView {
    /// Consume a key for the stdin notice or answer box. Returns `None` when
    /// the key keeps its normal routing.
    pub(super) fn handle_exec_stdin_prompt_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let (session, exec_session) = self.last_exec_stdin.clone()?;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if !self.stdin_box.is_focused_for(&session) {
            return match key.code {
                KeyCode::Char('o') if ctrl => {
                    self.stdin_box.focus(&session);
                    Some(EventResult::consumed())
                }
                KeyCode::Char('x') if ctrl => {
                    self.emit(Action::ExecStdinMuted {
                        agent_session: session,
                        exec_session,
                    });
                    Some(EventResult::consumed())
                }
                _ => None,
            };
        }
        match key.code {
            KeyCode::Char('c') if ctrl => {
                self.emit_exec_stdin_control(session, exec_session, ExecStdinControl::Interrupt);
            }
            KeyCode::Char('d') if ctrl => {
                self.emit_exec_stdin_control(session, exec_session, ExecStdinControl::Eof);
            }
            KeyCode::Esc if key.modifiers.is_empty() => self.stdin_box.unfocus(&session),
            KeyCode::Up if key.modifiers == KeyModifiers::SHIFT => {
                self.stdin_box.session_mut(&session).recall_older();
            }
            KeyCode::Down if key.modifiers == KeyModifiers::SHIFT => {
                self.stdin_box.session_mut(&session).recall_newer();
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                let state = self.stdin_box.session_mut(&session);
                let text = state.input.value();
                if !text.trim().is_empty() {
                    state.input.set_value("");
                    state.edited();
                    self.emit(Action::ExecStdinSubmit {
                        agent_session: session,
                        exec_session,
                        text,
                    });
                }
            }
            // Plain Enter never reaches the box input (handled above), so
            // its outcome is only ever Continued or Ignored.
            _ => self
                .stdin_box
                .session_mut(&session)
                .apply_event(&Event::Key(*key)),
        }
        Some(EventResult::consumed())
    }

    fn emit_exec_stdin_control(
        &self,
        agent_session: SessionId,
        exec_session: String,
        control: ExecStdinControl,
    ) {
        self.emit(Action::ExecStdinControl {
            agent_session,
            exec_session,
            control,
        });
    }

    /// Paste while the answer box is focused goes into the box. Returns
    /// `None` otherwise (the paste reaches the chat draft as usual).
    pub(super) fn handle_exec_stdin_prompt_paste(&mut self, text: &str) -> Option<EventResult> {
        let (session, _) = self
            .last_exec_stdin
            .clone()
            .filter(|(session, _)| self.stdin_box.is_focused_for(session))?;
        self.stdin_box
            .session_mut(&session)
            .apply_event(&Event::Paste(text.to_string()));
        Some(EventResult::consumed())
    }
}
