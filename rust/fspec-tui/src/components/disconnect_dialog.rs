//! Critical-priority Disconnect dialog (RPC-011 CR-1 baseline).
//!
//! Feature: spec/features/disconnect-dialog-cr1-baseline.feature
//! Feature: spec/features/rpc027-help-disconnect-thinking-dialogs.feature
//!
//! RPC-027: now renders via the shared dialog_theme renderer so the
//! red rounded border, bold red inner title, and black background
//! match the TypeScript reference. Behaviour (q/r/swallow nav keys)
//! is unchanged.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Stable id used by Compositor::remove on Action::Reconnected.
pub const DISCONNECT_DIALOG_ID: &str = "disconnect-dialog";

/// Critical-priority modal dialog shown when the WebSocketFspecBackend
/// loses its underlying connection.
pub struct DisconnectDialog {
    id: String,
    /// Current attempt count for the auto-reconnect supervisor; `None`
    /// means we have not yet seen an Action::Reconnecting (initial
    /// disconnect state).
    attempt: Option<u32>,
}

impl Default for DisconnectDialog {
    fn default() -> Self {
        Self {
            id: DISCONNECT_DIALOG_ID.to_string(),
            attempt: None,
        }
    }
}

impl DisconnectDialog {
    /// Construct a fresh DisconnectDialog with the canonical id and no
    /// active reconnect attempt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current auto-reconnect attempt counter, if any.
    /// `None` means no Action::Reconnecting has been observed yet.
    pub fn attempt(&self) -> Option<u32> {
        self.attempt
    }

    /// Render the body lines as a single static string. Public for
    /// tests that need to assert content without going through render().
    pub fn body(&self) -> String {
        let header = match self.attempt {
            None => "daemon disconnected".to_string(),
            Some(n) => format!("daemon disconnected — auto-reconnecting (attempt {n})…"),
        };
        format!("{header}\n\nq to quit\nr to reconnect")
    }

    fn body_rows(&self) -> Vec<DialogRow> {
        self.body()
            .lines()
            .map(|line| DialogRow {
                spans: vec![Span::raw(line.to_string())],
                selectable: false,
                selected: false,
            })
            .collect()
    }
}

impl Component for DisconnectDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    /// CR-1 rule [2]: while topmost, j/k/?/Tab are no-ops. ONLY 'q'
    /// (which emits Action::Quit via the App run loop's main dispatch)
    /// and 'r' (which emits Action::ManualReconnect) are honoured.
    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => {
                    let id = self.id.clone();
                    let callback: Callback = Box::new(move |compositor| {
                        let _ = compositor.remove(&id);
                    });
                    return EventResult::Consumed(Some(callback));
                }
                KeyCode::Char('r') => {
                    return EventResult::consumed();
                }
                KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('?') | KeyCode::Tab => {
                    return EventResult::consumed();
                }
                _ => {}
            }
        }
        // RPC-403 review: Critical modal — consume (swallow) pastes so
        // they can never leak into the agent input hidden behind this
        // dialog. No text field here, so nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn update(&mut self, action: Action) -> Option<Action> {
        if let Action::Reconnecting(n) = action {
            self.attempt = Some(n);
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog = FspecDialog {
            accent: Accent::Red,
            title: "Disconnected",
            rows: self.body_rows(),
            footer: "",
            min_width: 50,
            query_row: None,
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

    fn render_dialog_80x24(dialog: &mut DisconnectDialog) -> Buffer {
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
        let mut all_text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                all_text.push_str(buf[(x, y)].symbol());
            }
            all_text.push('\n');
        }
        all_text
    }

    #[test]
    fn dialog_priority_is_critical() {
        let dialog = DisconnectDialog::new();
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), DISCONNECT_DIALOG_ID);
    }

    #[test]
    fn dialog_renders_required_literal_strings() {
        let mut dialog = DisconnectDialog::new();
        let buf = render_dialog_80x24(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("daemon disconnected"));
        assert!(text.contains("q to quit"));
        assert!(text.contains("r to reconnect"));
    }

    #[test]
    fn reconnecting_action_updates_body_text_inline() {
        let mut dialog = DisconnectDialog::new();
        let _ = dialog.update(Action::Reconnecting(3));
        assert_eq!(dialog.attempt(), Some(3));
        let buf = render_dialog_80x24(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("auto-reconnecting (attempt 3)"));
    }

    #[test]
    fn disconnect_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let mut dialog = DisconnectDialog::new();
        let buf = render_dialog_80x24(&mut dialog);
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("disconnect_dialog__centered_popup_80x24", rows);
    }
}
