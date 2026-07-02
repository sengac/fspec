//! RPC-402 — Slash / file popup key routing + buffer re-classification,
//! extracted from `dispatch.rs` so the dispatch orchestrator stays
//! under the 300-LoC source-shape ceiling after gaining the
//! `KeyEventKind::Press` filter (RPC-402 rule [3]).
//!
//! Houses `handle_popup_key` (RPC-020 popup overlay routing),
//! `splice_path` (file-popup selection splicing), and `sync_popups`
//! (post-edit buffer re-classification).

use crossterm::event::KeyEvent;

use crate::components::{Action, EventResult};

use super::file_search_popup::{FilePopupOutcome, FileSearchPopup};
use super::popups::{classify_buffer, splice_file_selection, PopupTrigger};
use super::slash_command_popup::{PopupOutcome, SlashCommandPopup};
use super::AgentView;

impl AgentView {
    /// RPC-020: route the key through the slash / file popup overlays.
    pub(super) fn handle_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
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
        let new = splice_file_selection(
            &self.input.value(),
            anchor,
            filter_len,
            path,
            trailing_space,
        );
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
}
