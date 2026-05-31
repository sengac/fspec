//! AgentView event dispatch + popup/mode-view orchestration
//! (RPC-020 / RPC-026).
//!
//! Factored out of `views/agent.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Routing order:
//!   1. Ctrl+R chord — opens the search view when no popup / mode
//!      view is currently active (RPC-026).
//!   2. Resume / search MODE VIEW routing — when either is open the
//!      key event is consumed by the view before anything else.
//!   3. Slash / file popup routing (RPC-020).
//!   4. Default Esc/Ctrl+C/PageUp/Shift-arrow chord handling.
//!   5. Forward to MultiLineInput + `sync_popups` to refilter.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{Action, EventResult};

use super::file_search_popup::{FilePopupOutcome, FileSearchPopup};
use super::multiline_input::{InputEventOutcome, InputGate};
use super::popups::{classify_buffer, splice_file_selection, PopupTrigger};
use super::slash_command_popup::{PopupOutcome, SlashCommandPopup};
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

    /// RPC-020: route the key through the slash / file popup overlays.
    fn handle_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if let Some(popup) = self.slash_popup.as_mut() {
            match popup.handle_key(key.code, key.modifiers) {
                PopupOutcome::Selected(action) => {
                    self.slash_popup = None;
                    self.input.reset();
                    self.emit(Action::SlashCommandSelected(action));
                    return Some(EventResult::consumed());
                }
                PopupOutcome::Filled(text) => {
                    self.slash_popup = None;
                    self.input.set_value(&text);
                    return Some(EventResult::consumed());
                }
                PopupOutcome::Dismiss => {
                    self.slash_popup = None;
                    return Some(EventResult::consumed());
                }
                PopupOutcome::Continued => return Some(EventResult::consumed()),
                PopupOutcome::Ignored => {}
            }
        }
        if let Some(popup) = self.file_popup.as_mut() {
            match popup.handle_key(key.code, key.modifiers) {
                FilePopupOutcome::SelectedEnter(path) => {
                    let (anchor, filter_len) = (popup.anchor_offset(), popup.filter().len());
                    self.splice_path(anchor, filter_len, &path, true);
                    return Some(EventResult::consumed());
                }
                FilePopupOutcome::SelectedTab(path) => {
                    let (anchor, filter_len) = (popup.anchor_offset(), popup.filter().len());
                    self.splice_path(anchor, filter_len, &path, false);
                    return Some(EventResult::consumed());
                }
                FilePopupOutcome::Dismiss => {
                    self.file_popup = None;
                    return Some(EventResult::consumed());
                }
                FilePopupOutcome::Continued => return Some(EventResult::consumed()),
                FilePopupOutcome::Ignored => {}
            }
        }
        None
    }

    fn splice_path(&mut self, anchor: usize, filter_len: usize, path: &str, trailing_space: bool) {
        let new = splice_file_selection(&self.input.value(), anchor, filter_len, path, trailing_space);
        self.input.set_value(&new);
        self.file_popup = None;
    }

    /// RPC-020: re-classify the input buffer after an edit and
    /// open/close/refilter the popups accordingly.
    pub fn sync_popups(&mut self) {
        let trigger = classify_buffer(&self.input.value());
        match trigger {
            PopupTrigger::OpenSlash(filter) => {
                let popup = self
                    .slash_popup
                    .get_or_insert_with(SlashCommandPopup::default);
                if popup.filter() != filter {
                    popup.set_filter(&filter);
                }
                self.file_popup = None;
            }
            PopupTrigger::OpenFile { anchor, filter } => {
                let need_new = match self.file_popup.as_ref() {
                    Some(p) => p.anchor_offset() != anchor,
                    None => true,
                };
                if need_new {
                    self.file_popup = Some(FileSearchPopup::new(anchor, &filter));
                    self.emit(Action::SearchFiles(filter));
                } else if let Some(p) = self.file_popup.as_mut() {
                    if p.filter() != filter {
                        p.set_filter(&filter);
                        self.emit(Action::SearchFiles(filter));
                    }
                }
                self.slash_popup = None;
            }
            PopupTrigger::Close => {
                self.slash_popup = None;
                self.file_popup = None;
            }
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        // RPC-028: route mouse events to popups / mode views before
        // anything else. Implementation lives in `mouse_dispatch.rs`
        // so this file stays under the 300-LoC source-shape budget.
        if let Event::Mouse(m) = event {
            if let Some(result) = self.handle_mode_view_mouse(*m) {
                return result;
            }
            if let Some(result) = self.handle_popup_mouse(*m) {
                return result;
            }
            // RPC-094: wheel events that hit the scrollback rect.
            if let Some(result) = self.handle_scrollback_mouse(*m) {
                return result;
            }
            return EventResult::ignored();
        }
        if let Event::Key(key) = event {
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
            if key.code == KeyCode::Esc && key.modifiers.is_empty() {
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
                self.emit(Action::ScrollbackPageUp);
                return EventResult::consumed();
            }
            if key.code == KeyCode::PageDown || key.code == KeyCode::End {
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
        let outcome = match event {
            Event::Key(key) => self.input.handle_key_gated(key.code, key.modifiers, gate),
            other => self.input.handle_event(other),
        };
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
        if h == 0 { 10 } else { h }
    }

    pub(super) fn mode_view_visible_rows(&self) -> usize {
        match self.last_render_area {
            Some(area) => area.height.saturating_sub(3) as usize,
            None => 20,
        }
    }
}
