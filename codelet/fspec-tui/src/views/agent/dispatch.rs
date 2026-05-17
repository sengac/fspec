//! AgentView event dispatch + popup orchestration (RPC-020 / RPC-026).
//!
//! Factored out of `views/agent.rs` so the orchestrator file stays
//! under the 300-LoC ceiling (rule [10] / RPC-002 invariant). Contains
//! the key-routing flow used by `AgentView::handle_event`:
//!   1. Route the key through the resume / search popups first
//!      (RPC-026), then through the slash / file popups (RPC-020).
//!      Mutual exclusivity guarantees at most one popup is live, but
//!      the ordering matches architecture note [1] of RPC-026.
//!   2. Fall back to the default Esc/Ctrl+C/PageUp/Shift-arrow chord
//!      handling that lived inline before RPC-020.
//!   3. Forward to MultiLineInput and re-classify the buffer via
//!      `sync_popups` so popups open/close/refilter in lockstep with
//!      the post-edit buffer.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{Action, EventResult};

use super::file_search_popup::{FilePopupOutcome, FileSearchPopup};
use super::multiline_input::InputEventOutcome;
use super::popups::{classify_buffer, splice_file_selection, PopupTrigger};
use super::resume_picker::ResumePickerOutcome;
use super::search_palette::SearchPaletteOutcome;
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

    /// RPC-020/RPC-026: route the key through the popup (if any)
    /// BEFORE the normal input/dispatch flow. Returns
    /// `Some(EventResult)` when a popup handled the event; `None` means
    /// the caller should fall through to the default key-handling path.
    ///
    /// Order matters and matches the mutual-exclusivity rules:
    ///   1. RPC-026 resume picker (opened via `/resume`).
    ///   2. RPC-026 search palette (opened via `/search`).
    ///   3. RPC-020 slash popup.
    ///   4. RPC-020 file search popup.
    fn handle_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        if let Some(result) = self.handle_resume_popup_key(key) {
            return Some(result);
        }
        if let Some(result) = self.handle_search_popup_key(key) {
            return Some(result);
        }
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

    /// RPC-026: route a key through the resume picker popup. Returns
    /// `Some(EventResult)` when the popup is open AND the key was
    /// handled or explicitly continued; `None` when no popup is open
    /// or the popup ignored the key (so the caller falls through to
    /// later popups / default handling).
    fn handle_resume_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let popup = self.resume_popup.as_mut()?;
        match popup.handle_key(key.code, key.modifiers) {
            ResumePickerOutcome::Selected(session_id) => {
                self.resume_popup = None;
                self.emit(Action::AttachToSession(session_id));
                Some(EventResult::consumed())
            }
            ResumePickerOutcome::Dismiss => {
                self.resume_popup = None;
                Some(EventResult::consumed())
            }
            ResumePickerOutcome::Continued => Some(EventResult::consumed()),
            ResumePickerOutcome::Ignored => None,
        }
    }

    /// RPC-026: route a key through the search palette popup. Returns
    /// `Some(EventResult)` when the popup is open AND the key was
    /// handled or explicitly continued; `None` when no popup is open
    /// or the popup ignored the key.
    fn handle_search_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let popup = self.search_popup.as_mut()?;
        match popup.handle_key(key.code, key.modifiers) {
            SearchPaletteOutcome::FilterChanged(query) => {
                self.emit(Action::SearchHistory(query));
                Some(EventResult::consumed())
            }
            SearchPaletteOutcome::Selected(text) => {
                self.search_popup = None;
                self.emit(Action::InsertIntoInput(text));
                Some(EventResult::consumed())
            }
            SearchPaletteOutcome::Dismiss => {
                self.search_popup = None;
                Some(EventResult::consumed())
            }
            SearchPaletteOutcome::Continued => Some(EventResult::consumed()),
            SearchPaletteOutcome::Ignored => None,
        }
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
        if let Event::Key(key) = event {
            // RPC-020/RPC-026: route through the popup overlay BEFORE
            // the default Esc/Ctrl+C/PageUp/Shift-arrow chords. Popup
            // keys (↑↓/Enter/Tab/Esc) take precedence so a visible
            // popup is dismissed first.
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
        // RPC-020: re-classify after every input event so popups
        // open/close/refilter in lockstep with the buffer.
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
        // Use the last observed scrollback inner height when known;
        // fall back to a conservative default for first-frame events.
        let h = self.last_scrollback_viewport as usize;
        if h == 0 { 10 } else { h }
    }
}
