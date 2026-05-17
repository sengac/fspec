//! RPC-020 — File search popup widget.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user types `@` followed by zero-or-more non-space chars.
//! `filter` tracks the text after the `@`; `anchor_offset` records the
//! byte offset of the `@` in the joined buffer so the eventual
//! select+splice can locate and replace the correct `@<filter>`
//! substring.
//!
//! Search results are populated asynchronously: AgentView emits
//! `Action::SearchFiles(prefix)` after each filter change; the App
//! dispatch fires a tokio task calling `backend.search_files`, then
//! emits `Action::FileSearchResults(matches)` which AgentView folds
//! back into this popup via `set_matches`.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use tui_popup::Popup;

use super::popup_body::{widest_line, PopupBody};

/// Outcome of routing a single key event through the file search popup.
#[derive(Debug, Clone)]
pub enum FilePopupOutcome {
    /// User picked a path with Enter — splice it into the input,
    /// followed by a single trailing space.
    SelectedEnter(String),
    /// User picked a path with Tab — splice without a trailing space.
    SelectedTab(String),
    Dismiss,
    Continued,
    Ignored,
}

pub struct FileSearchPopup {
    filter: String,
    /// Byte offset of the `@` in the joined input buffer at the moment
    /// the popup opened. Used by AgentView's splice math.
    anchor_offset: usize,
    matches: Vec<String>,
    selected_index: usize,
}

impl FileSearchPopup {
    pub fn new(anchor_offset: usize, filter: &str) -> Self {
        Self {
            filter: filter.to_string(),
            anchor_offset,
            matches: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn anchor_offset(&self) -> usize {
        self.anchor_offset
    }

    pub fn matches(&self) -> &[String] {
        &self.matches
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&str> {
        self.matches.get(self.selected_index).map(String::as_str)
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Update the filter text. AgentView is expected to also emit
    /// `Action::SearchFiles(filter)` so a fresh result set arrives via
    /// `set_matches`.
    pub fn set_filter(&mut self, new_filter: &str) {
        if self.filter != new_filter {
            self.filter = new_filter.to_string();
            self.selected_index = 0;
        }
    }

    /// Replace the result list. Selection is clamped to the new length.
    pub fn set_matches(&mut self, matches: Vec<String>) {
        self.matches = matches;
        if self.matches.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.matches.len() {
            self.selected_index = self.matches.len() - 1;
        }
    }

    fn move_up(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.matches.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        if self.selected_index + 1 >= self.matches.len() {
            self.selected_index = 0;
        } else {
            self.selected_index += 1;
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> FilePopupOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            return FilePopupOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => FilePopupOutcome::Dismiss,
            KeyCode::Up => {
                self.move_up();
                FilePopupOutcome::Continued
            }
            KeyCode::Down => {
                self.move_down();
                FilePopupOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(path) => FilePopupOutcome::SelectedEnter(path.to_string()),
                None => FilePopupOutcome::Ignored,
            },
            KeyCode::Tab => match self.selected() {
                Some(path) => FilePopupOutcome::SelectedTab(path.to_string()),
                None => FilePopupOutcome::Ignored,
            },
            _ => FilePopupOutcome::Ignored,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let body = self.build_body();
        let width = widest_line(&body) + 2;
        let height = body.lines().count() as u16;
        let sized = PopupBody {
            text: body,
            selected_index: self.selected_index,
            width,
            height,
        };
        Popup::new(sized).title("File Search").render(area, buf);
    }

    fn build_body(&self) -> String {
        if self.matches.is_empty() {
            let label = if self.filter.is_empty() {
                "(type to search files)".to_string()
            } else {
                format!("(no files match \"{filter}\")", filter = self.filter)
            };
            return label;
        }
        let mut out = String::new();
        for (i, path) in self.matches.iter().take(10).enumerate() {
            let marker = if i == self.selected_index { "▸" } else { " " };
            out.push_str(&format!("{marker} {path}\n"));
        }
        out.push_str("\n↑↓ Navigate │ Tab/Enter Select │ Esc Close");
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn new_popup_has_no_matches_and_zero_index() {
        let p = FileSearchPopup::new(5, "rea");
        assert_eq!(p.anchor_offset(), 5);
        assert_eq!(p.filter(), "rea");
        assert_eq!(p.match_count(), 0);
        assert_eq!(p.selected_index(), 0);
    }

    #[test]
    fn set_matches_clamps_selection() {
        let mut p = FileSearchPopup::new(0, "");
        p.set_matches(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(p.match_count(), 3);
    }

    #[test]
    fn enter_selects_current_match_for_splice() {
        let mut p = FileSearchPopup::new(6, "rea");
        p.set_matches(vec!["README.md".to_string()]);
        match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
            FilePopupOutcome::SelectedEnter(path) => assert_eq!(path, "README.md"),
            other => panic!("expected SelectedEnter, got {other:?}"),
        }
    }

    #[test]
    fn tab_selects_current_match_for_partial_fill() {
        let mut p = FileSearchPopup::new(6, "rea");
        p.set_matches(vec!["README.md".to_string()]);
        match p.handle_key(KeyCode::Tab, KeyModifiers::NONE) {
            FilePopupOutcome::SelectedTab(path) => assert_eq!(path, "README.md"),
            other => panic!("expected SelectedTab, got {other:?}"),
        }
    }

    #[test]
    fn down_wraps_around() {
        let mut p = FileSearchPopup::new(0, "");
        p.set_matches(vec!["a".to_string(), "b".to_string()]);
        p.handle_key(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(p.selected_index(), 1);
        p.handle_key(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(p.selected_index(), 0);
    }

    #[test]
    fn enter_with_no_matches_is_ignored() {
        let mut p = FileSearchPopup::new(0, "");
        match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
            FilePopupOutcome::Ignored => {}
            other => panic!("expected Ignored, got {other:?}"),
        }
    }
}
