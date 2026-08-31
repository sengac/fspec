//! RPC-079 — Reusable Priority::Critical ErrorDialog wrapper.
//!
//! Feature: spec/features/rust-error-notification-status-dialog-wrappers.feature
//!
//! Direct Rust port of `src/components/ErrorDialog.tsx`. Renders a
//! centred red-bordered modal with a bold red `"Error"` title, the
//! caller-supplied message in red, and a dim centred `"Press ESC to
//! dismiss"` footer. Sticky (no auto-dismiss); ESC emits a
//! [`Callback`] that pops the dialog off the [`Compositor`].
//!
//! Every render delegates to [`dialog_theme::render_dialog`] so the
//! visual contract stays byte-equal to the rest of the dialog stack.

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Callback, Component, EventResult, Priority};

/// Canonical id used by [`crate::compositor::Compositor::remove`] when
/// the dialog dismisses.
pub const ERROR_DIALOG_ID: &str = "error-dialog";

/// Priority::Critical modal dialog displaying a single error message
/// with sticky ESC-only dismissal.
pub struct ErrorDialog {
    id: String,
    message: String,
}

impl ErrorDialog {
    /// Construct a fresh ErrorDialog with the canonical id and the
    /// supplied error message body.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: ERROR_DIALOG_ID.to_string(),
            message: message.into(),
        }
    }

    /// Read-only accessor for the error message body.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Component for ErrorDialog {
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
        // RPC-403 review: Critical modal — consume (swallow) pastes so
        // they can never leak into the agent input hidden behind this
        // dialog. No text field here, so nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let body_style = Style::default().fg(Color::Red).bg(Color::Black);
        let rows = vec![DialogRow {
            spans: vec![Span::styled(self.message.clone(), body_style)],
            selectable: false,
            selected: false,
        }];
        let dialog = FspecDialog {
            accent: Accent::Red,
            title: "Error",
            rows,
            footer: "Press ESC to dismiss",
            min_width: 40,
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

    fn render_to_buffer(dialog: &mut ErrorDialog) -> Buffer {
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
    fn error_dialog_is_critical_priority_with_canonical_id() {
        let dialog = ErrorDialog::new("boom");
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), ERROR_DIALOG_ID);
    }

    #[test]
    fn error_dialog_renders_required_literal_strings() {
        let mut dialog = ErrorDialog::new("Disk full");
        let buf = render_to_buffer(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("Error"), "buffer must contain Error title");
        assert!(text.contains("Disk full"), "buffer must contain body");
        assert!(
            text.contains("Press ESC to dismiss"),
            "buffer must contain dismiss footer"
        );
    }

    #[test]
    fn error_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let mut dialog = ErrorDialog::new("Disk full");
        let buf = render_to_buffer(&mut dialog);
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("error_dialog__centered_popup_80x24", rows);
    }
}
