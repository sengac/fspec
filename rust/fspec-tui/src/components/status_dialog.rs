//! RPC-079 — Reusable Priority::Critical StatusDialog wrapper.
//!
//! Feature: spec/features/rust-error-notification-status-dialog-wrappers.feature
//!
//! Direct Rust port of `src/components/StatusDialog.tsx`. A state
//! machine modelling a long-running batch operation:
//!
//!   - [`StatusKind::Restoring`] — Accent::Cyan border, cyan title
//!     ("`<operation_type>` Files"), current item body + `(idx/total)`
//!     counter. ESC is IGNORED in this state (no `Callback` emitted).
//!   - [`StatusKind::Complete`]  — Accent::Cyan border, GREEN title
//!     ("`<verb>` Complete!"), 3-second auto-close with live
//!     `"Closing in Ns... (ESC to dismiss)"` footer. ESC skips the
//!     wait and dismisses immediately.
//!   - [`StatusKind::Error`]     — Accent::Red border, bold red
//!     "Error" title, red error_message body, static
//!     `"Press ESC to dismiss"` footer. ESC dismisses.
//!
//! Every render delegates to [`dialog_theme::render_dialog`] so the
//! visual contract stays byte-equal to the rest of the dialog stack.

use std::time::Duration;

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by [`crate::compositor::Compositor::remove`].
pub const STATUS_DIALOG_ID: &str = "status-dialog";

/// Default auto-close timeout for [`StatusKind::Complete`] in
/// milliseconds — matches the TS reference (`StatusDialog.tsx` line
/// 53).
pub const COMPLETE_AUTO_CLOSE_MS: u64 = 3000;

/// Discriminator for the [`StatusDialog`] state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusKind {
    /// In-progress: current item, 1-based index, and total count.
    Restoring {
        current: String,
        idx: usize,
        total: usize,
    },
    /// Operation finished successfully. Auto-closes after
    /// [`COMPLETE_AUTO_CLOSE_MS`].
    Complete,
    /// Operation failed with the supplied error message body.
    Error { error_message: String },
}

/// Priority::Critical modal state-machine dialog for long-running
/// batch operations.
pub struct StatusDialog {
    id: String,
    operation_type: String,
    state: StatusKind,
    /// When Some(_), marks the instant we entered Complete state. Used
    /// for the live countdown footer.
    complete_at: Option<Instant>,
    action_tx: Option<UnboundedSender<Action>>,
    dismissal_task: Option<JoinHandle<()>>,
}

impl StatusDialog {
    /// Construct a fresh StatusDialog in [`StatusKind::Restoring`]
    /// state with empty progress fields and the supplied operation
    /// type verb (default "Restoring").
    pub fn new(operation_type: impl Into<String>) -> Self {
        Self {
            id: STATUS_DIALOG_ID.to_string(),
            operation_type: operation_type.into(),
            state: StatusKind::Restoring {
                current: String::new(),
                idx: 0,
                total: 0,
            },
            complete_at: None,
            action_tx: None,
            dismissal_task: None,
        }
    }

    /// Attach the App's [`Action`] channel. Required for
    /// [`Self::transition_to_complete`] to arm its auto-dismiss timer.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self
    }

    /// Read-only accessor for the current state.
    pub fn state(&self) -> &StatusKind {
        &self.state
    }

    /// Read-only accessor for the operation type verb (e.g.
    /// "Restoring").
    pub fn operation_type(&self) -> &str {
        &self.operation_type
    }

    /// Mutate the Restoring progress payload.
    pub fn set_restoring(&mut self, current: impl Into<String>, idx: usize, total: usize) {
        self.state = StatusKind::Restoring {
            current: current.into(),
            idx,
            total,
        };
    }

    /// Transition into [`StatusKind::Complete`] state and arm the
    /// 3-second auto-dismiss timer if an action channel is attached.
    pub fn transition_to_complete(&mut self) {
        self.state = StatusKind::Complete;
        self.complete_at = Some(Instant::now());
        self.arm_complete_auto_close();
    }

    /// Transition into [`StatusKind::Error`] state with the supplied
    /// error message body. Aborts any in-flight complete-state
    /// auto-close task.
    pub fn transition_to_error(&mut self, error_message: impl Into<String>) {
        self.abort_dismissal_task();
        self.complete_at = None;
        self.state = StatusKind::Error {
            error_message: error_message.into(),
        };
    }

    /// Test accessor — remaining ceiling-seconds shown in the Complete
    /// state countdown footer. Returns 0 outside Complete state.
    pub fn remaining_complete_seconds(&self) -> u64 {
        let Some(t0) = self.complete_at else {
            return 0;
        };
        let elapsed_ms = t0.elapsed().as_millis() as u64;
        let remaining_ms = COMPLETE_AUTO_CLOSE_MS.saturating_sub(elapsed_ms);
        remaining_ms.div_ceil(1000)
    }

    fn arm_complete_auto_close(&mut self) {
        let Some(tx) = self.action_tx.clone() else {
            return;
        };
        self.abort_dismissal_task();
        let id = self.id.clone();
        let delay = Duration::from_millis(COMPLETE_AUTO_CLOSE_MS);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(Action::DismissDialog(id));
        });
        self.dismissal_task = Some(handle);
    }

    fn abort_dismissal_task(&mut self) {
        if let Some(task) = self.dismissal_task.take() {
            task.abort();
        }
    }

    fn dismissal_callback(&self) -> Callback {
        let id = self.id.clone();
        Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        })
    }

    fn completion_verb(&self) -> String {
        // Strip a trailing "ing" so "Restoring" -> "Restore".
        if let Some(stem) = self.operation_type.strip_suffix("ing") {
            format!("{stem}e")
        } else {
            self.operation_type.clone()
        }
    }
}

impl Drop for StatusDialog {
    fn drop(&mut self) {
        self.abort_dismissal_task();
    }
}

impl Component for StatusDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Esc {
                match &self.state {
                    // Rule [7]: ESC ignored during Restoring.
                    StatusKind::Restoring { .. } => return EventResult::ignored(),
                    StatusKind::Complete | StatusKind::Error { .. } => {
                        self.abort_dismissal_task();
                        return EventResult::Consumed(Some(self.dismissal_callback()));
                    }
                }
            }
        }
        // RPC-403 review: Critical modal — consume (swallow) pastes so
        // they can never leak into the agent input hidden behind this
        // dialog (even while Restoring). Nothing is inserted.
        if matches!(event, Event::Paste(_)) {
            return EventResult::consumed();
        }
        EventResult::ignored()
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self.state.clone() {
            StatusKind::Restoring {
                current,
                idx,
                total,
            } => {
                let rows = vec![
                    DialogRow {
                        spans: vec![Span::raw(current)],
                        selectable: false,
                        selected: false,
                    },
                    DialogRow {
                        spans: vec![Span::raw(format!("({idx}/{total})"))],
                        selectable: false,
                        selected: false,
                    },
                ];
                let title_buf = format!("{} Files", self.operation_type);
                let dialog = FspecDialog {
                    accent: Accent::Cyan,
                    title: &title_buf,
                    rows,
                    footer: "",
                    min_width: 50,
query_row: None,
                };
                render_dialog(area, buf, &dialog);
            }
            StatusKind::Complete => {
                let remaining = self.remaining_complete_seconds();
                let footer_buf = format!("Closing in {remaining}s... (ESC to dismiss)");
                let title_buf = format!("{} Complete!", self.completion_verb());
                let dialog = FspecDialog {
                    accent: Accent::Cyan,
                    title: &title_buf,
                    rows: vec![DialogRow {
                        spans: vec![Span::raw(String::new())],
                        selectable: false,
                        selected: false,
                    }],
                    footer: &footer_buf,
                    min_width: 50,
query_row: None,
                };
                render_dialog(area, buf, &dialog);
                // Repaint title row to GREEN (border stays Cyan).
                overlay_title_color(area, buf, &title_buf, Color::Green);
            }
            StatusKind::Error { error_message } => {
                let body_style = Style::default().fg(Color::Red).bg(Color::Black);
                let rows = vec![DialogRow {
                    spans: vec![Span::styled(error_message, body_style)],
                    selectable: false,
                    selected: false,
                }];
                let dialog = FspecDialog {
                    accent: Accent::Red,
                    title: "Error",
                    rows,
                    footer: "Press ESC to dismiss",
                    min_width: 50,
query_row: None,
                };
                render_dialog(area, buf, &dialog);
            }
        }
    }
}

/// Overlay the title-row text with the supplied [`Color`], preserving
/// the existing background and bold modifier set by
/// [`dialog_theme::render_dialog`]. Identical helper to the one in
/// `notification_dialog.rs` — duplicated to keep the modules
/// independent (each is small + self-contained).
fn overlay_title_color(area: Rect, buf: &mut Buffer, title: &str, color: Color) {
    let title_chars: Vec<char> = title.chars().collect();
    if title_chars.is_empty() {
        return;
    }
    let y_end = area.y.saturating_add(area.height);
    let x_end = area.x.saturating_add(area.width);
    for y in area.y..y_end {
        for start_x in area.x..x_end {
            if start_x as usize + title_chars.len() > x_end as usize {
                break;
            }
            let matches = title_chars
                .iter()
                .enumerate()
                .all(|(i, ch)| buf[(start_x + i as u16, y)].symbol() == ch.to_string());
            if matches {
                for (i, _ch) in title_chars.iter().enumerate() {
                    let cell = &mut buf[(start_x + i as u16, y)];
                    let new_style = cell.style().fg(color);
                    cell.set_style(new_style);
                }
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    fn render_to_buffer(dialog: &mut StatusDialog) -> Buffer {
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
    fn status_dialog_is_critical_priority_with_canonical_id() {
        let dialog = StatusDialog::new("Restoring");
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), STATUS_DIALOG_ID);
    }

    #[test]
    fn status_dialog_restoring_renders_title_body_and_counter() {
        let mut dialog = StatusDialog::new("Restoring");
        dialog.set_restoring("file3.txt", 3, 10);
        let buf = render_to_buffer(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("Restoring Files"), "missing title");
        assert!(text.contains("file3.txt"), "missing current item");
        assert!(text.contains("(3/10)"), "missing counter");
    }

    #[test]
    fn status_dialog_error_renders_red_title_and_body() {
        let mut dialog = StatusDialog::new("Restoring");
        dialog.transition_to_error("Operation failed: read-only filesystem");
        let buf = render_to_buffer(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("Error"));
        assert!(text.contains("Operation failed: read-only filesystem"));
        assert!(text.contains("Press ESC to dismiss"));
    }

    #[test]
    fn status_dialog_restoring_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let mut dialog = StatusDialog::new("Restoring");
        dialog.set_restoring("file3.txt", 3, 10);
        let buf = render_to_buffer(&mut dialog);
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("status_dialog_restoring__centered_popup_80x24", rows);
    }

    #[test]
    fn status_dialog_error_rendering_is_byte_equal_across_runs_insta_snapshot() {
        let mut dialog = StatusDialog::new("Restoring");
        dialog.transition_to_error("Operation failed: read-only filesystem");
        let buf = render_to_buffer(&mut dialog);
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!("status_dialog_error__centered_popup_80x24", rows);
    }
}
