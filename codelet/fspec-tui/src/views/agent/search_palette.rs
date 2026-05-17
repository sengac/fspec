//! RPC-026 — Search palette popup widget.
//!
//! Feature: spec/features/rpc026-search-palette.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user picks the `/search` slash command. Typeahead-filters
//! across the HistoryMatch list returned by the backend's
//! `persistence_search_history` RPC method (lifted in RPC-025).
//!
//! Widget state:
//!   - `query`: String (the filter; chars/backspace edit it).
//!   - `matches`: Vec<HistoryMatch> (refreshed via set_matches).
//!   - `selected_index`: usize (clamped to matches.len()).
//!
//! Every typing event emits `FilterChanged(query)` so the App layer
//! can spawn a fresh `persistence_search_history` call. ↑/↓ navigate
//! with wrap-around; Enter emits `Selected(text)` with the highlighted
//! match's text (App routes Action::InsertIntoInput); Esc dismisses.

use codelet_rpc_types::HistoryMatch;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use tui_popup::Popup;

use super::popup_body::{widest_line, PopupBody};

/// Outcome of routing a single key event through the search palette.
#[derive(Debug, Clone)]
pub enum SearchPaletteOutcome {
    /// Query text changed — App should fire Action::SearchHistory(q).
    FilterChanged(String),
    /// User picked a match with Enter — App should fire
    /// Action::InsertIntoInput(text).
    Selected(String),
    /// User pressed Esc — drop the popup, leave input alone.
    Dismiss,
    /// Popup handled the key internally (navigation / no-op backspace).
    Continued,
    /// Popup ignored the key — caller may route it elsewhere.
    Ignored,
}

/// Search palette typeahead state.
pub struct SearchPalette {
    query: String,
    matches: Vec<HistoryMatch>,
    selected_index: usize,
}

impl Default for SearchPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPalette {
    /// Construct a fresh palette with empty query and no matches.
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&HistoryMatch> {
        self.matches.get(self.selected_index)
    }

    pub fn matches(&self) -> &[HistoryMatch] {
        &self.matches
    }

    /// Replace the filter text and reset selection to the first row.
    pub fn set_query(&mut self, new_query: &str) {
        self.query = new_query.to_string();
        self.selected_index = 0;
    }

    /// Replace the typeahead match list. Selection is clamped.
    pub fn set_matches(&mut self, matches: Vec<HistoryMatch>) {
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

    /// Route a single key event through the popup.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> SearchPaletteOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            return SearchPaletteOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => SearchPaletteOutcome::Dismiss,
            KeyCode::Up => {
                self.move_up();
                SearchPaletteOutcome::Continued
            }
            KeyCode::Down => {
                self.move_down();
                SearchPaletteOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(m) => SearchPaletteOutcome::Selected(m.text.clone()),
                None => SearchPaletteOutcome::Ignored,
            },
            KeyCode::Backspace => {
                if self.query.is_empty() {
                    SearchPaletteOutcome::Continued
                } else {
                    self.query.pop();
                    self.selected_index = 0;
                    SearchPaletteOutcome::FilterChanged(self.query.clone())
                }
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.selected_index = 0;
                SearchPaletteOutcome::FilterChanged(self.query.clone())
            }
            _ => SearchPaletteOutcome::Ignored,
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
        Popup::new(sized).title("Search History").render(area, buf);
    }

    fn build_body(&self) -> String {
        if self.matches.is_empty() {
            if self.query.is_empty() {
                return "(type to search history)".to_string();
            }
            return format!("(no history matches \"{query}\")", query = self.query);
        }
        let mut out = String::new();
        for (i, m) in self.matches.iter().take(10).enumerate() {
            let marker = if i == self.selected_index { "▸" } else { " " };
            out.push_str(&format!("{marker} {text}\n", text = m.text));
        }
        out.push_str("\n↑↓ Navigate │ Enter Insert │ Esc Close");
        out
    }
}
