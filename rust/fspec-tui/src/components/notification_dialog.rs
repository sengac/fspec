//! RPC-079 — Reusable Priority::Critical NotificationDialog wrapper.
//!
//! Feature: spec/features/rust-error-notification-status-dialog-wrappers.feature
//!
//! Direct Rust port of `src/components/NotificationDialog.tsx`. Renders
//! a centred bordered modal with a bold severity-coloured title, the
//! caller-supplied message, and a footer that EITHER shows a live
//! `"Closing in Ns... (ESC to dismiss)"` countdown (when
//! `auto_dismiss_ms > 0`) OR a static `"Press ESC to dismiss"` (when
//! `auto_dismiss_ms == 0`).
//!
//! Severity → accent mapping (matches the TS reference):
//!
//!   - [`NotificationSeverity::Success`] → [`Accent::Cyan`] border,
//!     [`Color::Green`] bold title "Success"
//!   - [`NotificationSeverity::Info`]    → [`Accent::Cyan`] border,
//!     [`Color::Cyan`] bold title "Info"
//!   - [`NotificationSeverity::Warning`] → [`Accent::Yellow`] border,
//!     [`Color::Yellow`] bold title "Warning"
//!
//! Every render delegates to [`dialog_theme::render_dialog`] so the
//! visual contract stays byte-equal to the rest of the dialog stack.

use std::time::Duration;

use crossterm::event::{Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::{Action, Callback, Component, EventResult, Priority};

/// Canonical id used by [`crate::compositor::Compositor::remove`].
pub const NOTIFICATION_DIALOG_ID: &str = "notification-dialog";

/// Severity of a [`NotificationDialog`]. Drives both the border accent
/// and the bold title colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSeverity {
    Success,
    Info,
    Warning,
}

impl NotificationSeverity {
    /// Map the severity to the dialog border [`Accent`].
    pub fn accent(self) -> Accent {
        match self {
            NotificationSeverity::Success | NotificationSeverity::Info => Accent::Cyan,
            NotificationSeverity::Warning => Accent::Yellow,
        }
    }

    /// Map the severity to the bold title text colour.
    pub fn title_color(self) -> Color {
        match self {
            NotificationSeverity::Success => Color::Green,
            NotificationSeverity::Info => Color::Cyan,
            NotificationSeverity::Warning => Color::Yellow,
        }
    }

    /// Map the severity to the literal title text used in the dialog.
    pub fn title_text(self) -> &'static str {
        match self {
            NotificationSeverity::Success => "Success",
            NotificationSeverity::Info => "Info",
            NotificationSeverity::Warning => "Warning",
        }
    }
}

/// Default auto-dismiss timeout in milliseconds — matches the TS
/// reference (`NotificationDialog.tsx` line 57).
pub const DEFAULT_AUTO_DISMISS_MS: u64 = 2000;

/// Priority::Critical modal dialog displaying a success / info /
/// warning notification with an optional auto-dismiss countdown.
pub struct NotificationDialog {
    id: String,
    message: String,
    severity: NotificationSeverity,
    auto_dismiss_ms: u64,
    created_at: Instant,
    action_tx: Option<UnboundedSender<Action>>,
    dismissal_task: Option<JoinHandle<()>>,
    /// Cached footer text. Rebuilt on every render so the countdown
    /// reflects the live elapsed time.
    footer_buf: String,
}

impl NotificationDialog {
    /// Construct a fresh NotificationDialog with the supplied
    /// message + severity. Defaults `auto_dismiss_ms` to
    /// [`DEFAULT_AUTO_DISMISS_MS`] (2000ms). No timer task is spawned
    /// until [`Self::with_action_tx`] attaches an action channel.
    pub fn new(message: impl Into<String>, severity: NotificationSeverity) -> Self {
        Self {
            id: NOTIFICATION_DIALOG_ID.to_string(),
            message: message.into(),
            severity,
            auto_dismiss_ms: DEFAULT_AUTO_DISMISS_MS,
            created_at: Instant::now(),
            action_tx: None,
            dismissal_task: None,
            footer_buf: String::new(),
        }
    }

    /// Convenience constructor — Success severity.
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, NotificationSeverity::Success)
    }

    /// Convenience constructor — Info severity.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationSeverity::Info)
    }

    /// Convenience constructor — Warning severity.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, NotificationSeverity::Warning)
    }

    /// Override the auto-dismiss delay in milliseconds. `0` disables
    /// auto-dismiss entirely and switches the footer to the static
    /// `"Press ESC to dismiss"` text.
    pub fn with_auto_dismiss_ms(mut self, ms: u64) -> Self {
        self.auto_dismiss_ms = ms;
        self
    }

    /// Attach the App's [`Action`] channel and arm the auto-dismiss
    /// timer (when `auto_dismiss_ms > 0`). Spawns a `tokio::spawn`
    /// task that sleeps then sends [`Action::DismissDialog`] with the
    /// dialog's stable id; the task is aborted if ESC dismisses first.
    pub fn with_action_tx(mut self, action_tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(action_tx);
        self.arm_auto_dismiss();
        self
    }

    /// Read-only accessor for the message body.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Read-only accessor for the severity.
    pub fn severity(&self) -> NotificationSeverity {
        self.severity
    }

    /// Read-only accessor for the auto-dismiss timeout in ms.
    pub fn auto_dismiss_ms(&self) -> u64 {
        self.auto_dismiss_ms
    }

    /// Test accessor — current remaining ceiling-seconds. Returns the
    /// integer shown in the countdown footer.
    pub fn remaining_seconds(&self) -> u64 {
        let elapsed_ms = self.created_at.elapsed().as_millis() as u64;
        let remaining_ms = self.auto_dismiss_ms.saturating_sub(elapsed_ms);
        remaining_ms.div_ceil(1000)
    }

    /// Spawn the dismissal task if both [`auto_dismiss_ms`] and
    /// [`action_tx`] are present. Idempotent — replaces any previously
    /// armed task. Must be called from inside a tokio runtime.
    fn arm_auto_dismiss(&mut self) {
        if self.auto_dismiss_ms == 0 {
            return;
        }
        let Some(tx) = self.action_tx.clone() else {
            return;
        };
        if let Some(prev) = self.dismissal_task.take() {
            prev.abort();
        }
        let id = self.id.clone();
        let delay = Duration::from_millis(self.auto_dismiss_ms);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = tx.send(Action::DismissDialog(id));
        });
        self.dismissal_task = Some(handle);
    }

    fn abort_auto_dismiss(&mut self) {
        if let Some(task) = self.dismissal_task.take() {
            task.abort();
        }
    }

    fn body_row(&self) -> DialogRow {
        DialogRow {
            spans: vec![Span::raw(self.message.clone())],
            selectable: false,
            selected: false,
        }
    }

    fn build_footer(&mut self) -> &str {
        if self.auto_dismiss_ms == 0 {
            self.footer_buf.clear();
            self.footer_buf.push_str("Press ESC to dismiss");
        } else {
            let remaining = self.remaining_seconds();
            self.footer_buf.clear();
            self.footer_buf
                .push_str(&format!("Closing in {remaining}s... (ESC to dismiss)"));
        }
        &self.footer_buf
    }
}

impl Drop for NotificationDialog {
    fn drop(&mut self) {
        self.abort_auto_dismiss();
    }
}

impl Component for NotificationDialog {
    fn priority(&self) -> Priority {
        Priority::Critical
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        if let Event::Key(key) = event {
            if key.code == KeyCode::Esc {
                self.abort_auto_dismiss();
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
        let title_style = Style::default()
            .fg(self.severity.title_color())
            .add_modifier(Modifier::BOLD)
            .bg(Color::Black);
        let _ = title_style; // styling applied by dialog_theme via accent
        let rows = vec![self.body_row()];
        // The `dialog_theme::render_dialog` paints the title using
        // `accent.color()`. To honour the severity-specific
        // title_color (Success → Green) which differs from the
        // border accent (Success → Cyan), we render the dialog with
        // the title_color as accent THEN paint the border back to the
        // severity accent. The cleanest approach is to compute footer
        // and rows here, and to use a small two-pass render: first
        // we render with the title-color accent (to get a Green
        // title for Success severity), then we OVERWRITE the border
        // cells with the proper severity accent.
        //
        // However, this gets messy. Simpler: render with the
        // SEVERITY ACCENT for the border, but pass the title text
        // wrapped in a styled Span pre-coloured to title_color. The
        // dialog_theme renderer always re-styles the title row
        // bold + accent.color() (see dialog_theme.rs lines 164–173).
        // That means we'd have to bypass the renderer's title row.
        //
        // Pragmatic approach (matches the TS reference): for the
        // Success variant, the border is GREEN (title_color) and the
        // title is also GREEN (since both come from `color` in the TS
        // `NotificationDialog.tsx`). Re-reading the TS source:
        //   `<Dialog borderColor={color} ...>`
        //   `<Text bold color={color}>{title}</Text>`
        // So in TS the border AND title are the SAME colour per
        // severity. The Gherkin reformulation in RPC-079 introduces
        // a SEPARATE accent: Success → Cyan border / Green title;
        // Info → Cyan border / Cyan title; Warning → Yellow border /
        // Yellow title. The Cyan border for Success is the only
        // divergence from TS — it matches the "Accent::Cyan with
        // green title text" rule from the work-unit description.
        //
        // To implement the divergent Success styling we render the
        // dialog with `Accent::Cyan` (border) but paint the title
        // cell-by-cell AFTER the render_dialog call.
        let dialog = FspecDialog {
            accent: self.severity.accent(),
            title: self.severity.title_text(),
            rows,
            footer: self.build_footer(),
            min_width: 40,
query_row: None,
        };
        render_dialog(area, buf, &dialog);

        // Repaint the title row with the severity-specific
        // title_color (only different from accent.color() for the
        // Success severity → Green title atop a Cyan border).
        if self.severity == NotificationSeverity::Success {
            overlay_title_color(area, buf, self.severity.title_text(), Color::Green);
        }
    }
}

/// Overlay the title-row text with the supplied [`Color`], preserving
/// the existing background and bold modifier set by
/// [`dialog_theme::render_dialog`]. We locate the title by scanning
/// for the exact `title` string inside the buffer rows that lie
/// inside `area`. Walks at most `area.height` rows and stops at the
/// first match — guaranteed by `render_dialog` to be on row
/// `inner.y + 1` (i.e. the second row inside the border).
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

    fn render_to_buffer(dialog: &mut NotificationDialog) -> Buffer {
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
    fn notification_dialog_is_critical_priority_with_canonical_id() {
        let dialog = NotificationDialog::success("hi");
        assert_eq!(dialog.priority(), Priority::Critical);
        assert_eq!(dialog.id(), NOTIFICATION_DIALOG_ID);
    }

    #[test]
    fn notification_dialog_renders_title_and_body() {
        let mut dialog = NotificationDialog::success("Saved");
        let buf = render_to_buffer(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("Success"), "missing Success title");
        assert!(text.contains("Saved"), "missing body");
    }

    #[test]
    fn notification_dialog_warning_static_footer_when_auto_dismiss_zero() {
        let mut dialog = NotificationDialog::warning("Slow").with_auto_dismiss_ms(0);
        let buf = render_to_buffer(&mut dialog);
        let text = buffer_text(&buf);
        assert!(text.contains("Warning"));
        assert!(text.contains("Slow"));
        assert!(
            text.contains("Press ESC to dismiss"),
            "auto_dismiss_ms=0 must show static footer"
        );
        assert!(
            !text.contains("Closing in"),
            "auto_dismiss_ms=0 must NOT show countdown footer"
        );
    }

    #[test]
    fn notification_dialog_rendering_is_byte_equal_across_runs_insta_snapshot() {
        // Use auto_dismiss_ms=0 for a deterministic footer.
        let mut dialog = NotificationDialog::success("Saved").with_auto_dismiss_ms(0);
        let buf = render_to_buffer(&mut dialog);
        let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
        for y in 0..buf.area.height {
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            rows.push(row);
        }
        insta::assert_yaml_snapshot!(
            "notification_dialog_success_no_autoclose__centered_popup_80x24",
            rows
        );
    }
}
