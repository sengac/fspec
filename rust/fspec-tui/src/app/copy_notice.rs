//! TUI-126: the copy confirmation every `Action::CopyToClipboard` shows.
//!
//! Feature: spec/features/sign-in-screens-copy-open-paste-and-wrap.feature
//!
//! OSC 52 gives no answer, so without a confirmation every copy looks the
//! same as a dead key. `App::handle_copy_to_clipboard` sets the notice after
//! each write; the App paints it on the bottom row, right-aligned, over
//! whatever view or dialog is shown, for [`COPY_NOTICE_FOR`] on tokio time.
//! TUI-130 routes every copy through the same path, so every copy confirms
//! the same way.
//!
//! TUI-130: the notice says only what the App knows. The App knows it wrote
//! the OSC 52 sequence to the terminal (or why the write failed); it cannot
//! know whether the terminal passed it on to the system clipboard (tmux
//! without `set-clipboard on` drops it), so a written copy says "Sent N
//! characters to the terminal clipboard (OSC 52)", never "Copied".

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use tokio::time::Instant;
use unicode_width::UnicodeWidthStr;

/// How long a copy confirmation stays on screen.
pub const COPY_NOTICE_FOR: Duration = Duration::from_secs(2);

/// The confirmation of the latest copy, until it expires.
#[derive(Debug, Default)]
pub(crate) struct CopyNotice {
    shown: Option<(String, bool, Instant)>,
}

impl CopyNotice {
    /// Confirm a copy of `text` that `result` says was written or not.
    pub(crate) fn record(&mut self, text: &str, result: &std::io::Result<()>) {
        let (message, ok) = match result {
            Ok(()) => (
                format!(
                    "✓ Sent {} characters to the terminal clipboard (OSC 52)",
                    text.chars().count()
                ),
                true,
            ),
            Err(error) => (format!("✗ Copy failed: {error}"), false),
        };
        self.shown = Some((message, ok, Instant::now() + COPY_NOTICE_FOR));
    }

    /// The confirmation text while it is shown.
    pub(crate) fn message(&self) -> Option<&str> {
        self.shown
            .as_ref()
            .filter(|(_, _, until)| Instant::now() < *until)
            .map(|(message, _, _)| message.as_str())
    }

    /// True while the run loop must keep drawing: the notice is shown, or it
    /// just expired and one more frame has to clear it (then it is dropped).
    pub(crate) fn needs_redraw(&mut self) -> bool {
        match &self.shown {
            Some(_) if self.message().is_some() => true,
            Some(_) => {
                self.shown = None;
                true
            }
            None => false,
        }
    }

    /// Paint the notice on the bottom row of `area`, right-aligned.
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer) {
        let (Some(message), Some((_, ok, _))) = (self.message(), self.shown.as_ref()) else {
            return;
        };
        if area.height == 0 || area.width == 0 {
            return;
        }
        let text = format!(" {message} ");
        let width = (text.width() as u16).min(area.width);
        let row = Rect {
            x: area.x + area.width - width,
            y: area.y + area.height - 1,
            width,
            height: 1,
        };
        let style = Style::default()
            .fg(Color::Black)
            .bg(if *ok { Color::Green } else { Color::Red })
            .add_modifier(Modifier::BOLD);
        Paragraph::new(Line::styled(text, style)).render(row, buf);
    }
}
