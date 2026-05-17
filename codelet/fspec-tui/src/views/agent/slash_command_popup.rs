//! RPC-020 — Slash command popup widget.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user types a leading `/`. Filter text tracks the
//! characters after the `/`; ↑/↓ navigate (wrap-around); Enter
//! selects+executes; Tab fills the input without executing; Esc
//! dismisses. Other keys propagate so the user can keep typing.
//!
//! Ownership: AgentView holds an `Option<SlashCommandPopup>`. When
//! `Some`, AgentView routes its keystrokes through `handle_key` BEFORE
//! forwarding to MultiLineInput. `AgentView::sync_popups` invokes
//! `set_filter` after every input event so the list reflects the
//! post-edit buffer.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use tui_popup::Popup;

use super::popup_body::{widest_line, PopupBody};
use super::slash_commands::{filter_commands, SlashCommand, SlashCommandAction, SLASH_COMMANDS};

/// Outcome of routing a single key event through the slash popup.
///
/// `Selected` carries the action the App should dispatch; AgentView
/// drops the popup before propagating. `Filled(text)` requests
/// MultiLineInput to set its buffer to `text` AND drop the popup
/// without executing. `Dismiss` closes the popup but leaves the input
/// unchanged. `Continued` means the popup handled the key internally
/// (navigation). `Ignored` means the popup ignored the key — the
/// caller should forward to MultiLineInput.
#[derive(Debug, Clone)]
pub enum PopupOutcome {
    Selected(SlashCommandAction),
    Filled(String),
    Dismiss,
    Continued,
    Ignored,
}

/// Slash command palette state.
pub struct SlashCommandPopup {
    filter: String,
    matches: Vec<&'static SlashCommand>,
    selected_index: usize,
}

impl Default for SlashCommandPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandPopup {
    /// Construct a fresh popup with an empty filter and the first match
    /// pre-selected.
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            matches: SLASH_COMMANDS.iter().collect(),
            selected_index: 0,
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn matches(&self) -> &[&'static SlashCommand] {
        &self.matches
    }

    pub fn selected(&self) -> Option<&'static SlashCommand> {
        self.matches.get(self.selected_index).copied()
    }

    /// Update the filter (text after the leading `/`). Resets the
    /// selection to index 0 so the top match is highlighted.
    pub fn set_filter(&mut self, new_filter: &str) {
        self.filter = new_filter.to_string();
        self.matches = filter_commands(&self.filter);
        self.selected_index = 0;
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

    /// Route a single key event through the popup. The caller (AgentView)
    /// invokes this BEFORE forwarding to MultiLineInput.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> PopupOutcome {
        if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::CONTROL) {
            // Shift+arrow / Ctrl+anything reserved for AgentView's
            // navigation chords — never the popup's.
            return PopupOutcome::Ignored;
        }
        match code {
            KeyCode::Esc => PopupOutcome::Dismiss,
            KeyCode::Up => {
                self.move_up();
                PopupOutcome::Continued
            }
            KeyCode::Down => {
                self.move_down();
                PopupOutcome::Continued
            }
            KeyCode::Enter => match self.selected() {
                Some(cmd) => PopupOutcome::Selected(cmd.action),
                None => PopupOutcome::Dismiss,
            },
            KeyCode::Tab => match self.selected() {
                Some(cmd) => PopupOutcome::Filled(format!("/{}", cmd.name())),
                None => PopupOutcome::Dismiss,
            },
            _ => PopupOutcome::Ignored,
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
        Popup::new(sized).title("Slash Commands").render(area, buf);
    }

    fn build_body(&self) -> String {
        if self.matches.is_empty() {
            return "(no matching commands)".to_string();
        }
        let max_name = self
            .matches
            .iter()
            .map(|c| c.name().len())
            .max()
            .unwrap_or(8);
        let mut out = String::new();
        for (i, cmd) in self.matches.iter().take(10).enumerate() {
            let marker = if i == self.selected_index { "▸" } else { " " };
            out.push_str(&format!(
                "{marker} /{name:<width$}  {desc}\n",
                marker = marker,
                name = cmd.name(),
                width = max_name,
                desc = cmd.description
            ));
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
    fn new_popup_has_full_registry_and_first_selected() {
        let p = SlashCommandPopup::new();
        assert_eq!(p.match_count(), SLASH_COMMANDS.len());
        assert_eq!(p.selected_index(), 0);
        assert_eq!(p.filter(), "");
        assert!(p.selected().is_some());
    }

    #[test]
    fn set_filter_narrows_matches() {
        let mut p = SlashCommandPopup::new();
        p.set_filter("he");
        assert!(p.match_count() >= 1);
        assert_eq!(p.matches()[0].name(), "help");
        assert_eq!(p.selected_index(), 0);
    }

    #[test]
    fn down_wraps_around() {
        let mut p = SlashCommandPopup::new();
        for _ in 0..p.match_count() {
            p.handle_key(KeyCode::Down, KeyModifiers::NONE);
        }
        assert_eq!(p.selected_index(), 0);
    }

    #[test]
    fn up_wraps_to_end() {
        let mut p = SlashCommandPopup::new();
        p.handle_key(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(p.selected_index(), p.match_count() - 1);
    }

    #[test]
    fn enter_emits_selected_action() {
        let mut p = SlashCommandPopup::new();
        match p.handle_key(KeyCode::Enter, KeyModifiers::NONE) {
            PopupOutcome::Selected(a) => assert_eq!(a, SlashCommandAction::Help),
            other => panic!("expected Selected, got {other:?}"),
        }
    }

    #[test]
    fn tab_returns_filled_with_command_name() {
        let mut p = SlashCommandPopup::new();
        p.set_filter("c");
        match p.handle_key(KeyCode::Tab, KeyModifiers::NONE) {
            PopupOutcome::Filled(s) => assert!(s.starts_with('/'), "got: {s}"),
            other => panic!("expected Filled, got {other:?}"),
        }
    }

    #[test]
    fn esc_returns_dismiss() {
        let mut p = SlashCommandPopup::new();
        match p.handle_key(KeyCode::Esc, KeyModifiers::NONE) {
            PopupOutcome::Dismiss => {}
            other => panic!("expected Dismiss, got {other:?}"),
        }
    }

    #[test]
    fn ordinary_char_is_ignored_so_caller_can_route_to_input() {
        let mut p = SlashCommandPopup::new();
        match p.handle_key(KeyCode::Char('q'), KeyModifiers::NONE) {
            PopupOutcome::Ignored => {}
            other => panic!("expected Ignored, got {other:?}"),
        }
    }
}
