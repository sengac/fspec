//! RPC-406 — pause-prompt key routing for the AgentView.
//!
//! Feature: spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Mirrors the TS HIGH-priority pause handler
//! (`AgentView.tsx:4521-4607`): while the FOCUSED session has an
//! active pause slot (cached on the view as `last_pause` at render
//! time), EVERY key is consumed here so nothing reaches the
//! MultiLineInput — the draft stays untouched. Ctrl+C still emits
//! `Action::Interrupt` (session interrupt outranks the prompt).
//!
//! | Kind    | Key       | Action                                     |
//! |---------|-----------|--------------------------------------------|
//! | triple  | ← / →     | `PausePromptNav { delta: ∓1 }` (wraparound)|
//! | triple  | Enter     | `PausePromptEnter` (store-authoritative)   |
//! | triple  | Esc       | `PauseTriple { choice: Deny }`             |
//! | confirm | Y / y     | `PauseConfirmed { accept: true }`          |
//! | confirm | N/n / Esc | `PauseConfirmed { accept: false }`         |
//!
//! Esc NEVER resumes — the pause-resume action is not constructible
//! from this module (source-shape locked by
//! `the_pause_modal_is_deleted_and_resume_is_unreachable_from_the_prompt`).
//! All actions target the PAUSED session id from the cache, not the
//! focused index at key time.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use codelet_rpc_types::{ApprovalChoice, PauseKind};

use crate::components::{Action, EventResult};

use super::AgentView;

impl AgentView {
    /// Consume a key event for the active pause prompt. Returns `None`
    /// when no pause prompt is showing for the focused session
    /// (normal routing continues).
    pub(super) fn handle_pause_prompt_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let (session, kind) = self.last_pause.clone()?;
        // Ctrl+C outranks the prompt — session interrupt stays live.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.emit(Action::Interrupt);
            return Some(EventResult::consumed());
        }
        match kind {
            PauseKind::Triple => match key.code {
                KeyCode::Left => {
                    self.emit(Action::PausePromptNav {
                        session_id: session,
                        delta: -1,
                    });
                }
                KeyCode::Right => {
                    self.emit(Action::PausePromptNav {
                        session_id: session,
                        delta: 1,
                    });
                }
                KeyCode::Enter => {
                    self.emit(Action::PausePromptEnter {
                        session_id: session,
                    });
                }
                KeyCode::Esc => {
                    // Security fix: Esc DENIES (TS AgentView.tsx:4593-4600).
                    self.emit(Action::PauseTriple {
                        session_id: session,
                        choice: ApprovalChoice::Deny,
                    });
                }
                // Swallow everything else — no key reaches the input.
                _ => {}
            },
            PauseKind::Confirm => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.emit(Action::PauseConfirmed {
                        session_id: session,
                        accept: true,
                    });
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    // Esc cancels = deny (TS AgentView.tsx:4558-4565).
                    self.emit(Action::PauseConfirmed {
                        session_id: session,
                        accept: false,
                    });
                }
                _ => {}
            },
        }
        Some(EventResult::consumed())
    }
}
