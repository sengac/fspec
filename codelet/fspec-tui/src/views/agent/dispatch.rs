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
use super::multiline_input::InputEventOutcome;
use super::popups::{classify_buffer, splice_file_selection, PopupTrigger};
use super::resume_session_view::ResumeSessionViewOutcome;
use super::search_history_view::SearchHistoryViewOutcome;
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
    /// Returns `Some(EventResult)` when a mode view consumed the key.
    fn handle_mode_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if let Some(result) = self.handle_resume_view_key(key) {
            return Some(result);
        }
        if let Some(result) = self.handle_search_view_key(key) {
            return Some(result);
        }
        None
    }

    fn handle_resume_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let visible_rows = self.mode_view_visible_rows();
        let view = self.resume_view.as_mut()?;
        match view.handle_key(key.code, key.modifiers, visible_rows) {
            ResumeSessionViewOutcome::Selected(session_id) => {
                self.resume_view = None;
                self.emit(Action::AttachToSession(session_id));
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::Dismiss => {
                self.resume_view = None;
                self.emit(Action::CloseResumeView);
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::RequestDelete(session_id) => {
                self.emit(Action::RequestDeleteSession(session_id));
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::ConfirmedDelete(session_id) => {
                self.emit(Action::ConfirmDeleteSession(session_id));
                Some(EventResult::consumed())
            }
            ResumeSessionViewOutcome::CancelledDelete => Some(EventResult::consumed()),
            ResumeSessionViewOutcome::Continued => Some(EventResult::consumed()),
            ResumeSessionViewOutcome::Ignored => Some(EventResult::consumed()),
        }
    }

    fn handle_search_view_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let visible_rows = self.mode_view_visible_rows();
        let view = self.search_view.as_mut()?;
        match view.handle_key(key.code, key.modifiers, visible_rows) {
            SearchHistoryViewOutcome::FilterChanged(query) => {
                self.emit(Action::SearchHistory(query));
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Selected(text) => {
                self.search_view = None;
                self.emit(Action::InsertIntoInput(text));
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Dismiss => {
                self.search_view = None;
                self.emit(Action::CloseSearchView);
                Some(EventResult::consumed())
            }
            SearchHistoryViewOutcome::Continued => Some(EventResult::consumed()),
            SearchHistoryViewOutcome::Ignored => Some(EventResult::consumed()),
        }
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
                self.emit(Action::BackToBoard);
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
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                if let Some(action) = Self::shift_arrow_to_action(key.code) {
                    self.emit(action);
                    return EventResult::consumed();
                }
            }
        }
        let outcome = self.input.handle_event(event);
        self.sync_popups();
        match outcome {
            InputEventOutcome::Submitted(value) => {
                if value.is_empty() {
                    return EventResult::ignored();
                }
                self.emit(Action::InputSubmitted(value));
                EventResult::consumed()
            }
            InputEventOutcome::Continued => EventResult::consumed(),
            InputEventOutcome::Ignored => EventResult::ignored(),
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
