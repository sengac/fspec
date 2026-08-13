//! RPC-381 — AgentView SELECT-mode key routing, split out of
//! `dispatch.rs` so that file stays under the 300-LoC source-shape
//! ceiling pinned by `rpc094-agentview-scrollback-scroll.feature`.
//!
//! Feature: spec/features/agentview-turn-select-mode.feature

use crossterm::event::{KeyCode, KeyEvent};

use crate::components::{Action, EventResult};

use super::AgentView;

impl AgentView {
    /// RPC-381: SELECT-mode key routing (only consulted when
    /// `turn_select_mode` is active). Up/Down emit turn navigation,
    /// Enter is suppressed (no submit), and Esc exits the mode locally
    /// (consumed, no `AgentEscPressed`). Any other key falls through to
    /// the normal handling by returning `None`.
    pub(super) fn handle_turn_select_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if !key.modifiers.is_empty() {
            return None;
        }
        match key.code {
            KeyCode::Up => {
                // RPC-382/383: while the modal is open, Up scrolls the
                // modal body up — the underlying selection must NOT move.
                if self.turn_modal_seq.is_some() {
                    self.emit(Action::TurnModalScrollUp);
                } else {
                    self.emit(Action::TurnNavUp);
                }
                Some(EventResult::consumed())
            }
            KeyCode::Down => {
                if self.turn_modal_seq.is_some() {
                    self.emit(Action::TurnModalScrollDown);
                } else {
                    self.emit(Action::TurnNavDown);
                }
                Some(EventResult::consumed())
            }
            KeyCode::PageUp if self.turn_modal_seq.is_some() => {
                self.emit(Action::TurnModalPageUp);
                Some(EventResult::consumed())
            }
            KeyCode::PageDown if self.turn_modal_seq.is_some() => {
                self.emit(Action::TurnModalPageDown);
                Some(EventResult::consumed())
            }
            KeyCode::Home if self.turn_modal_seq.is_some() => {
                self.emit(Action::TurnModalHome);
                Some(EventResult::consumed())
            }
            KeyCode::End if self.turn_modal_seq.is_some() => {
                self.emit(Action::TurnModalEnd);
                Some(EventResult::consumed())
            }
            KeyCode::Enter => {
                // RPC-382: Enter on the selected turn opens the turn
                // content modal (replaces the RPC-381 suppression). The
                // App reducer resolves the turn `seq` from the focused
                // scrollback's selection. If the modal is already open,
                // just consume.
                if self.turn_modal_seq.is_none() {
                    self.emit(Action::OpenTurnModal);
                }
                Some(EventResult::consumed())
            }
            KeyCode::Esc => {
                // RPC-382 Esc cascade: close the modal first (stay in
                // SELECT mode); only a second Esc exits SELECT mode.
                if self.turn_modal_seq.is_some() {
                    // COPY-008 rule [6]: the FIRST Esc clears an active
                    // modal text selection (modal stays open); a later
                    // Esc then closes the modal.
                    if self.turn_modal_selection.is_some() {
                        self.turn_modal_selection = None;
                        return Some(EventResult::consumed());
                    }
                    self.turn_modal_seq = None;
                    self.emit(Action::CloseTurnModal);
                    return Some(EventResult::consumed());
                }
                self.turn_select_mode = false;
                self.emit(Action::ToggleTurnSelectMode);
                Some(EventResult::consumed())
            }
            _ => None,
        }
    }

    /// RPC-381: Tab toggles SELECT mode view-locally and emits the
    /// `ToggleTurnSelectMode` reducer mirror.
    ///
    /// RPC-382: when disabling SELECT mode, also clear `turn_modal_seq`
    /// view-locally — this mirrors the Esc tear-down (see `handle_turn_select_key`)
    /// and the App reducer disable path, so a standalone AgentView stays
    /// consistent (no orphaned turn-content modal). The clear only runs on
    /// the disable transition (new `turn_select_mode == false`).
    pub(super) fn handle_tab_toggle(&mut self) -> EventResult {
        self.turn_select_mode = !self.turn_select_mode;
        if !self.turn_select_mode {
            self.turn_modal_seq = None;
        }
        self.emit(Action::ToggleTurnSelectMode);
        EventResult::consumed()
    }
}
