//! Critical-priority Help dialog (RPC-008 rule [15]).
//!
//! Feature: spec/features/fspec-tui-help-dialog.feature
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//!
//! Triggered by the `?` key at App-level (NOT inside HelloComponent —
//! the App layer pushes this onto the compositor). Body lists exactly
//! the `?`, ESC, and `q` keybindings. ESC returns
//! `EventResult::Consumed(Some(callback))` where the callback removes
//! the dialog by id.
//!
//! RPC-027: now renders via the shared dialog_theme renderer so the
//! cyan border, bold inner title, and dim centered footer match the
//! TypeScript Ink reference exactly.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Callback, Component, EventResult, Priority};

const HELP_LINES: &[&str] = &[
    "j/k     Navigate",
    "Tab     Switch pane",
    "?       Toggle this help",
    "q       Quit fspec-tui",
    "Enter   Send",
    "Ctrl+C  Interrupt",
    "ESC     Dismiss this dialog",
];

/// Critical-priority modal dialog listing the App-level keybindings.
pub struct HelpDialog {
    id: String,
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self {
            id: "help-dialog".to_string(),
        }
    }
}

impl HelpDialog {
    /// Construct a HelpDialog with the canonical id `"help-dialog"`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for HelpDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Esc {
                let id = self.id.clone();
                let callback: Callback = Box::new(move |compositor| {
                    let _ = compositor.remove(&id);
                });
                return EventResult::Consumed(Some(callback));
            }
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows: Vec<DialogRow> = HELP_LINES
            .iter()
            .map(|line| DialogRow {
                spans: vec![Span::raw((*line).to_string())],
                selectable: false,
                selected: false,
            })
            .collect();
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: "Help",
            rows,
            footer: "ESC to close",
            min_width: 30,
        };
        render_dialog(area, buf, &dialog);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    fn render_help_dialog_80x24() -> Buffer {
        let mut dialog = HelpDialog::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                dialog.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn help_dialog_is_critical_priority_with_canonical_id() {
        let dialog = HelpDialog::new();
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), "help-dialog");
    }

    #[test]
    fn help_dialog_body_lists_the_rpc009_keybindings() {
        let buf = render_help_dialog_80x24();
        let text = buffer_text(&buf);
        for needle in &["j/k", "Tab", "?", "q", "Enter", "Ctrl+C", "ESC"] {
            assert!(text.contains(needle), "buffer must contain {needle}: {text}");
        }
    }

    #[test]
    fn help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let buf = render_help_dialog_80x24();
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("help_dialog__centered_popup_80x24", rows);
    }
}
