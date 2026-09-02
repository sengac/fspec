//! TOOL-022 P2 — exec-stdin-prompt key routing for the AgentView.
//!
//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! Mirrors the HITL freeform key handler (`hitl_keys.rs`) but
//! simpler — freeform only (no options machine). While the FOCUSED
//! session has an active exec-stdin slot (cached on the view as
//! `last_exec_stdin` at render time) this module is consulted AFTER
//! the HITL keys (HITL still wins) and BEFORE the pause keys:
//!
//!   Esc            → `ExecStdinDismissed` (dismiss the overlay only —
//!                      the session keeps running, NOTHING is sent)
//!   plain Enter    → empty/whitespace → ignored (stay on the prompt);
//!                    else the shared input value is read + cleared
//!                    here and carried in `ExecStdinSubmit { text }`
//!   typing / paste → routed directly into the shared MultiLineInput
//!                      (never through the composer's Enter-submits-
//!                      message path)
//!
//! Ctrl+C still emits `Action::Interrupt` (session interrupt stays
//! live — same precedence as the HITL prompt).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{Action, EventResult};

use super::multiline_input::InputGate;
use super::AgentView;

impl AgentView {
    /// Consume a key event for the active exec-stdin prompt. Returns
    /// `None` when no exec-stdin prompt is showing for the focused
    /// session.
    pub(super) fn handle_exec_stdin_prompt_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let (session, exec_session) = self.last_exec_stdin.clone()?;
        // Ctrl+C outranks the prompt — session interrupt stays live.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.emit(Action::Interrupt);
            return Some(EventResult::consumed());
        }
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            // Dismiss the overlay only: the session keeps running,
            // nothing is cancelled or killed, and the composer draft
            // the user had before the prompt appeared is preserved
            // (the shared input value is never cleared on Esc).
            self.emit(Action::ExecStdinDismissed {
                agent_session: session,
            });
            return Some(EventResult::consumed());
        }
        if key.code == KeyCode::Enter && key.modifiers.is_empty() {
            let value = self.input.value();
            if value.trim().is_empty() {
                // Stay on the prompt — an empty answer sends nothing.
                return Some(EventResult::consumed());
            }
            // Read + clear the SHARED input; the reducer stays
            // store-authoritative via ExecStdinSubmit.
            self.input.set_value("");
            self.emit(Action::ExecStdinSubmit {
                agent_session: session,
                exec_session,
                text: value,
            });
            return Some(EventResult::consumed());
        }
        // Everything else falls through to the shared MultiLineInput
        // (typing, arrows, Shift/Alt+Enter newline). Routed directly —
        // the AgentView chord handlers (history nav, session switch,
        // Tab select-mode) must not fire while the prompt is live.
        let _ = self
            .input
            .handle_event_gated(&Event::Key(*key), InputGate::default());
        Some(EventResult::consumed())
    }

    /// Paste routing while the exec-stdin prompt is live: inserts into
    /// the shared input. Returns `None` when no exec-stdin prompt is
    /// showing (normal paste routing continues).
    pub(super) fn handle_exec_stdin_prompt_paste(&mut self, text: &str) -> Option<EventResult> {
        let _ = self.last_exec_stdin.clone()?;
        let _ = self
            .input
            .handle_event_gated(&Event::Paste(text.to_string()), InputGate::default());
        Some(EventResult::consumed())
    }
}
