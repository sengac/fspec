//! RPC-411 — HITL-prompt key routing for the AgentView.
//!
//! Feature: spec/features/inline-hitl-prompt.feature
//!
//! Mirrors the TS HIGH-priority HITL handler (`useHitlInput.ts:153-262`):
//! while the FOCUSED session has an active HITL slot (cached on the
//! view as `last_hitl` at render time) this module is consulted BEFORE
//! `handle_pause_prompt_key` and all other routing.
//!
//! Options mode — EVERY key is consumed here (no hotkeys, no Tab
//! cycle, no scroll-select), paste included:
//!   ↑ / ↓  → `HitlPromptNav { delta: ∓1 }` (wraparound over
//!            options + the virtual Other...)
//!   Enter  → `HitlPromptEnter` (store-authoritative reducer)
//!   Esc    → `HitlCancelled` (cancels the WHOLE request — sends
//!            `{cancelled:true}`; the old modal's silent pop is gone)
//!
//! Freeform / Other mode — only Enter/Esc are intercepted; everything
//! else (typing, paste, modified Enter = newline) falls through to the
//! SHARED composer MultiLineInput:
//!   Esc          → `HitlOtherExit` (Other) / `HitlCancelled` (plain)
//!   plain Enter  → empty/whitespace → `HitlEmptySubmit`; else the
//!                  input value is read + cleared here and carried in
//!                  `HitlAnswerCaptured { text }`
//!   typing       → `HitlHintCleared` when the empty-submit hint shows
//!
//! Ctrl+C still emits `Action::Interrupt` in both modes.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use codelet_rpc_types::SessionId;

use crate::components::{Action, EventResult};

use super::multiline_input::InputGate;
use super::AgentView;

/// Render-time snapshot of the HITL slot mode, cached as
/// `AgentView::last_hitl` so key routing targets the prompt's session
/// (mirrors `last_pause`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HitlKeyMode {
    /// Radio-list options question — every key consumed.
    Options,
    /// Pure freeform question or the Other... sub-mode — the shared
    /// composer input is live.
    Freeform { other: bool, hint: bool },
}

impl AgentView {
    /// RPC-406/RPC-411 cursor gate: no hardware cursor inside the
    /// pause prompt or an options-mode HITL prompt; freeform/Other
    /// mode shows the cursor inside the SHARED composer input.
    pub(super) fn is_cursor_visible_with_prompts(
        &self,
        session_status: Option<codelet_rpc_types::SessionStatus>,
    ) -> bool {
        // TOOL-022 P2: the exec-stdin overlay is live — the shared
        // input is the target, so the cursor shows there even though
        // the session status stays Running (exec-stdin performs NO
        // status flip, which would otherwise gate the cursor off via
        // `is_cursor_visible_for`).
        if self.last_exec_stdin.is_some() {
            return true;
        }
        if let Some((_, mode)) = &self.last_hitl {
            if matches!(mode, HitlKeyMode::Options) {
                return false;
            }
            return Self::is_cursor_visible_for(session_status, &self.input_transition_state);
        }
        if self.last_pause.is_some() {
            return false;
        }
        Self::is_cursor_visible_for(session_status, &self.input_transition_state)
    }

    /// Consume a key event for the active HITL prompt. Returns `None`
    /// when no HITL prompt is showing for the focused session.
    pub(super) fn handle_hitl_prompt_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let (session, mode) = self.last_hitl.clone()?;
        // Ctrl+C outranks the prompt — session interrupt stays live.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.emit(Action::Interrupt);
            return Some(EventResult::consumed());
        }
        match mode {
            HitlKeyMode::Options => {
                self.handle_hitl_options_key(session, key);
                Some(EventResult::consumed())
            }
            HitlKeyMode::Freeform { other, hint } => {
                Some(self.handle_hitl_freeform_key(session, key, other, hint))
            }
        }
    }

    fn handle_hitl_options_key(&mut self, session: SessionId, key: &KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.emit(Action::HitlPromptNav {
                    session_id: session,
                    delta: -1,
                });
            }
            KeyCode::Down => {
                self.emit(Action::HitlPromptNav {
                    session_id: session,
                    delta: 1,
                });
            }
            KeyCode::Enter => {
                self.emit(Action::HitlPromptEnter {
                    session_id: session,
                });
            }
            KeyCode::Esc => {
                // Cancel the WHOLE request — the reducer SENDS
                // {cancelled:true} before clearing the slot.
                self.emit(Action::HitlCancelled {
                    session_id: session,
                });
            }
            // Swallow everything else — no hotkeys, no Tab cycle;
            // nothing reaches the composer input.
            _ => {}
        }
    }

    fn handle_hitl_freeform_key(
        &mut self,
        session: SessionId,
        key: &KeyEvent,
        other: bool,
        hint: bool,
    ) -> EventResult {
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            if other {
                // Local only: back to options; the shared input value
                // belongs to the abandoned answer — clear it here.
                self.input.set_value("");
                self.emit(Action::HitlOtherExit {
                    session_id: session,
                });
            } else {
                self.emit(Action::HitlCancelled {
                    session_id: session,
                });
            }
            return EventResult::consumed();
        }
        if key.code == KeyCode::Enter && key.modifiers.is_empty() {
            let value = self.input.value();
            if value.trim().is_empty() {
                self.emit(Action::HitlEmptySubmit {
                    session_id: session,
                });
            } else {
                // Read + clear the SHARED input; the reducer stays
                // store-authoritative via HitlAnswerCaptured.
                self.input.set_value("");
                self.emit(Action::HitlAnswerCaptured {
                    session_id: session,
                    text: value,
                });
            }
            return EventResult::consumed();
        }
        // Everything else falls through to the shared MultiLineInput
        // (typing, arrows, Shift/Alt+Enter newline). Routed directly —
        // the AgentView chord handlers (history nav, session switch,
        // Tab select-mode) must not fire while the prompt is live.
        let before = self.input.value();
        let _ = self
            .input
            .handle_event_gated(&Event::Key(*key), InputGate::default());
        if hint && self.input.value() != before {
            self.emit(Action::HitlHintCleared {
                session_id: session,
            });
        }
        EventResult::consumed()
    }

    /// Paste routing while the HITL prompt is live: options mode
    /// swallows the paste (composer draft untouched); freeform/Other
    /// mode inserts into the shared input. Returns `None` when no
    /// HITL prompt is showing (normal paste routing continues).
    pub(super) fn handle_hitl_prompt_paste(&mut self, text: &str) -> Option<EventResult> {
        let (session, mode) = self.last_hitl.clone()?;
        match mode {
            HitlKeyMode::Options => Some(EventResult::consumed()),
            HitlKeyMode::Freeform { hint, .. } => {
                let before = self.input.value();
                let _ = self
                    .input
                    .handle_event_gated(&Event::Paste(text.to_string()), InputGate::default());
                if hint && self.input.value() != before {
                    self.emit(Action::HitlHintCleared {
                        session_id: session,
                    });
                }
                Some(EventResult::consumed())
            }
        }
    }
}
