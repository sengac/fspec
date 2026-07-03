//! AgentView event dispatch + popup/mode-view orchestration
//! (RPC-020 / RPC-026).
//!
//! Factored out of `views/agent.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Routing order:
//!   0. Non-Press key events (Release/Repeat) are dropped up-front
//!      (RPC-402 rule [3] — kitty enhancement protocol / Windows).
//!   1. Ctrl+R chord — opens the search view when no popup / mode
//!      view is currently active (RPC-026).
//!   2. Resume / search MODE VIEW routing — when either is open the
//!      key event is consumed by the view before anything else.
//!   3. Slash / file popup routing (RPC-020, in `dispatch_popups.rs`).
//!   4. Default Esc/Ctrl+C/PageUp/Shift-arrow chord handling.
//!   5. Forward to MultiLineInput + `sync_popups` to refilter.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::components::{Action, EventResult};

use super::multiline_input::{InputEventOutcome, InputGate};
use super::AgentView;

impl AgentView {
    fn shift_arrow_to_action(code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Up => Some(Action::HistoryPrev),
            KeyCode::Down => Some(Action::HistoryNext),
            KeyCode::Left => Some(Action::SessionPrev),
            KeyCode::Right => Some(Action::SessionNext),
            _ => None,
        }
    }

    fn is_ctrl_r(key: &KeyEvent) -> bool {
        key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R'))
    }

    /// RPC-026: route the key through resume / search mode views FIRST.
    fn handle_mode_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if let Some(result) = self.handle_resume_view_key(key) {
            return Some(result);
        }
        if let Some(result) = self.handle_search_view_key(key) {
            return Some(result);
        }
        None
    }

    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        // RPC-028: route mouse events to popups / mode views first
        // (impl in `mouse_dispatch.rs` to keep this file under 300 LoC).
        if let Event::Mouse(m) = event {
            if let Some(result) = self.handle_mode_view_mouse(*m) {
                return result;
            }
            if let Some(result) = self.handle_popup_mouse(*m) {
                return result;
            }
            // RPC-383: while the turn modal is open, the wheel scrolls
            // the modal body (NOT the scrollback).
            if let Some(result) = self.handle_turn_modal_mouse(*m) {
                return result;
            }
            // RPC-094: wheel events that hit the scrollback rect.
            if let Some(result) = self.handle_scrollback_mouse(*m) {
                return result;
            }
            // COPY-007: left press/drag/release over the input rect feeds
            // the composer's own selection recognizer (before bubbling).
            if let Some(result) = self.handle_composer_mouse(*m) {
                return result;
            }
            return EventResult::ignored();
        }
        // RPC-411: paste routing while the HITL prompt is live —
        // options mode swallows, freeform/Other inserts into the
        // shared input.
        if let Event::Paste(text) = event {
            if let Some(result) = self.handle_hitl_prompt_paste(text) {
                return result;
            }
        }
        if let Event::Key(key) = event {
            // RPC-402 rule [3]: only KeyEventKind::Press key events are
            // processed. Release/Repeat events (delivered under the
            // kitty enhancement protocol, and unconditionally on
            // Windows) must not double-fire ANY branch below — the
            // Esc/Ctrl+C/PageUp/Tab/Shift-arrow chords or the
            // MultiLineInput itself.
            if key.kind != KeyEventKind::Press {
                return EventResult::ignored();
            }
            // RPC-411: the inline HITL prompt (focused session has an
            // active HITL slot, cached at render time) is consulted
            // BEFORE the pause prompt and all other routing. Options
            // mode consumes every key; freeform/Other mode intercepts
            // Enter/Esc and lets the rest reach the shared input.
            if let Some(result) = self.handle_hitl_prompt_key(key) {
                return result;
            }
            // RPC-406: the inline pause prompt (focused session has an
            // active pause slot, cached at render time) consumes EVERY
            // key before any other routing — nothing may reach the
            // MultiLineInput while the prompt is showing. Esc DENIES.
            if let Some(result) = self.handle_pause_prompt_key(key) {
                return result;
            }
            // RPC-026: Ctrl+R opens the search view when no popup /
            // mode view is currently active.
            if Self::is_ctrl_r(key)
                && self.resume_view.is_none()
                && self.search_view.is_none()
                && self.slash_popup.is_none()
                && self.file_popup.is_none()
            {
                self.emit(Action::OpenSearchView);
                return EventResult::consumed();
            }
            // RPC-026: mode views consume everything when active.
            if let Some(result) = self.handle_mode_view_key(key) {
                return result;
            }
            if let Some(result) = self.handle_popup_key(key) {
                return result;
            }
            // RPC-381: Tab toggles turn-selection (SELECT) mode. Placed
            // AFTER mode-view + popup routing so an open popup consumes
            // Tab first. The view-local flag flips here; the App reducer
            // mirrors it onto the per-session scrollback selection.
            if key.code == KeyCode::Tab && key.modifiers.is_empty() {
                return self.handle_tab_toggle();
            }
            // COPY-006 rule [10]: Esc first clears an active text
            // selection (the most transient/foreground state), BEFORE the
            // RPC-381 turn-select exit and the AgentEscPressed cascade.
            if self.text_selection_active
                && key.code == KeyCode::Esc
                && key.modifiers.is_empty()
            {
                self.text_selection_active = false;
                self.emit(Action::SelectionClear);
                return EventResult::consumed();
            }
            // RPC-381: in SELECT mode, ↑/↓ navigate turn-to-turn (not
            // line scroll), Enter is suppressed, Esc exits locally.
            if self.turn_select_mode {
                if let Some(result) = self.handle_turn_select_key(key) {
                    return result;
                }
            }
            if key.code == KeyCode::Esc && key.modifiers.is_empty() {
                // COPY-007 rule [5]: Esc first clears an active COMPOSER
                // selection (consume, no submit) before the scrollback
                // Esc-cascade / AgentEscPressed.
                if self.input.text_selection_active() {
                    self.input.clear_selection();
                    return EventResult::consumed();
                }
                // RPC-051 Esc-cascade: levels 1-3 consumed above;
                // levels 4-5 decided in App::dispatch (need backend).
                self.emit(Action::AgentEscPressed);
                return EventResult::consumed();
            }
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.emit(Action::Interrupt);
                return EventResult::consumed();
            }
            if key.code == KeyCode::PageUp {
                self.text_selection_active = false; // COPY-006 rule [7].
                self.emit(Action::ScrollbackPageUp);
                return EventResult::consumed();
            }
            if key.code == KeyCode::PageDown || key.code == KeyCode::End {
                self.text_selection_active = false; // COPY-006 rule [7].
                self.emit(Action::ScrollbackPageDown);
                return EventResult::consumed();
            }
            // RPC-094: Home on an empty input jumps scrollback to 0.
            if key.code == KeyCode::Home && key.modifiers.is_empty() && self.input.is_empty() {
                self.emit(Action::ScrollbackHome);
                return EventResult::consumed();
            }
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                if let Some(action) = Self::shift_arrow_to_action(key.code) {
                    self.emit(action);
                    return EventResult::consumed();
                }
            }
        }
        let before = self.input.value();
        // RPC-094: capture arrow direction for the Ignored branch.
        let arrow_kind = match event {
            Event::Key(k) if k.modifiers.is_empty() => match k.code {
                KeyCode::Up => Some(KeyCode::Up),
                KeyCode::Down => Some(KeyCode::Down),
                _ => None,
            },
            _ => None,
        };
        // RPC-095: compute the gate from cached session status +
        // popup state. block_edits while Compacting; suppress_enter
        // also true during Compacting (Enter must NOT submit).
        let gate = InputGate {
            block_edits: self.last_is_compacting,
            suppress_enter: self.last_is_compacting,
        };
        // RPC-402/RPC-403: single gated entry point — key events
        // reaching here are guaranteed Press (filtered above); the
        // widget's own kind filter is defense-in-depth for direct
        // `handle_event` callers.
        let outcome = self.input.handle_event_gated(event, gate);
        self.sync_popups();
        match outcome {
            InputEventOutcome::Submitted(value) => {
                if value.is_empty() {
                    return EventResult::ignored();
                }
                self.emit(Action::InputSubmitted(value));
                EventResult::consumed()
            }
            InputEventOutcome::Continued => {
                // RPC-052: emit PendingInputChanged ONLY when the
                // buffer text actually changed.
                let after = self.input.value();
                if after != before {
                    self.emit(Action::PendingInputChanged(after));
                }
                EventResult::consumed()
            }
            InputEventOutcome::Ignored => match arrow_kind {
                // RPC-094: arrow at textarea edge → scrollback line.
                Some(KeyCode::Up) => {
                    self.emit(Action::ScrollbackLineUp);
                    EventResult::consumed()
                }
                Some(KeyCode::Down) => {
                    self.emit(Action::ScrollbackLineDown);
                    EventResult::consumed()
                }
                _ => EventResult::ignored(),
            },
        }
    }

    pub(crate) fn scrollback_viewport_hint(&self) -> usize {
        let h = self.last_scrollback_viewport as usize;
        if h == 0 {
            10
        } else {
            h
        }
    }

    pub(super) fn mode_view_visible_rows(&self) -> usize {
        match self.last_render_area {
            Some(area) => area.height.saturating_sub(3) as usize,
            None => 20,
        }
    }
}
