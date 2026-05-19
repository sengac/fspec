//! RPC-020 + RPC-027 — Slash command popup widget.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//! Feature: spec/features/rpc027-slash-file-popups.feature
//!
//! Centred floating overlay rendered above AgentView's MultiLineInput
//! when the user types a leading `/`. Filter text tracks the
//! characters after the `/`; ↑/↓ navigate (wrap-around); Enter
//! selects+executes; Tab fills the input without executing; Esc
//! dismisses. RPC-027 renders via the shared dialog_theme renderer.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use super::slash_commands::{filter_commands, SlashCommand, SlashCommandAction, SLASH_COMMANDS};
use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, MARKER_SELECTED, MARKER_UNSELECTED,
};

/// Outcome of routing a single key event through the slash popup.
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
        let rows = self.build_rows();
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Slash Commands",
            rows,
            footer: "↑↓ Navigate │ Tab/Enter Select │ Esc Close",
            min_width: 45,
        };
        render_dialog(area, buf, &dialog);
    }

    fn build_rows(&self) -> Vec<DialogRow> {
        if self.matches.is_empty() {
            return vec![DialogRow {
                spans: vec![
                    Span::raw(MARKER_UNSELECTED.to_string()),
                    Span::raw("(no matching commands)".to_string()),
                ],
                selectable: false,
                selected: false,
            }];
        }
        let max_name = self
            .matches
            .iter()
            .map(|c| c.name().len())
            .max()
            .unwrap_or(8);
        let mut out = Vec::new();
        for (i, cmd) in self.matches.iter().take(10).enumerate() {
            let is_sel = i == self.selected_index;
            let marker = if is_sel {
                MARKER_SELECTED
            } else {
                MARKER_UNSELECTED
            };
            let name_token = format!("/{name:<width$}", name = cmd.name(), width = max_name);
            let mut spans: Vec<Span<'static>> = vec![
                Span::raw(marker.to_string()),
                Span::raw(name_token),
                Span::raw("  ".to_string()),
            ];
            if is_sel {
                spans.push(Span::raw(cmd.description.to_string()));
            } else {
                spans.push(Span::styled(
                    cmd.description.to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
            out.push(DialogRow {
                spans,
                selectable: true,
                selected: is_sel,
            });
        }
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

    #[test]
    fn slash_command_popup_rendering_is_byte_equal_across_runs_insta_snapshot() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let popup = SlashCommandPopup::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                popup.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("slash_command_popup__centered_popup_80x24", rows);
    }
}
