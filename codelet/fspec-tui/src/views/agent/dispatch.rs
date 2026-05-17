//! AgentView event dispatch + popup orchestration (RPC-020).
//!
//! Factored out of `views/agent.rs` so the orchestrator file stays
//! under the 300-LoC ceiling (rule [10] / RPC-002 invariant). Contains
//! the key-routing flow used by `AgentView::handle_event`:
//!   1. Route the key through the slash / file popup first.
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

    /// RPC-020: route the key through the popup (if any) BEFORE the
    /// normal input/dispatch flow. Returns `Some(EventResult)` when the
    /// popup handled the event; `None` means the caller should fall
    /// through to the default key-handling path.
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
        if let Event::Key(key) = event {
            // RPC-020: route through the popup overlay BEFORE the
            // default Esc/Ctrl+C/PageUp/Shift-arrow chords. Popup keys
            // (↑↓/Enter/Tab/Esc) take precedence so a visible popup is
            // dismissed first.
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
                self.scrollback.scroll_up(self.scrollback_viewport_hint());
                return EventResult::consumed();
            }
            if key.code == KeyCode::PageDown || key.code == KeyCode::End {
                self.scrollback.scroll_down(self.scrollback_viewport_hint());
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

    pub(super) fn scrollback_viewport_hint(&self) -> usize {
        10
    }
}
