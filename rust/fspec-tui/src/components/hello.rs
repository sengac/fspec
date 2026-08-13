//! Background-priority placeholder component (RPC-008 rule [14]).
//!
//! Feature: spec/features/fspec-tui-hello.feature
//!
//! Renders a centered static greeting via `Layout::vertical([Min, Length,
//! Min])` + `Layout::horizontal([Min, Length, Min])` — the doc 06
//! centered-modal helper pattern. Never consumes events; ignored events
//! propagate through to whichever Critical-priority modal sits on top.
//!
//! Replaced by the real list view in RPC-009.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use super::{Component, Priority};

/// Static greeting text — kept short so it always fits inside the middle
/// row band even on narrow terminals.
const HELLO_TEXT: &str = "fspec-tui (RPC-008 placeholder)";

/// Background-priority placeholder Component.
pub struct HelloComponent {
    id: String,
}

impl Default for HelloComponent {
    fn default() -> Self {
        Self {
            id: "hello".to_string(),
        }
    }
}

impl HelloComponent {
    /// Construct a HelloComponent with the canonical id `"hello"`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for HelloComponent {
    fn priority(&self) -> Priority {
        Priority::Background
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Vertical centering: pad above + below leaves a single
        // Length(1) row in the middle for the greeting.
        let [_top, mid, _bottom] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);

        // Horizontal centering: pad left + right leaves a single
        // Length(N) column band sized to the greeting.
        let len = HELLO_TEXT.width() as u16;
        let [_left, center, _right] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(len),
            Constraint::Min(0),
        ])
        .areas(mid);

        Paragraph::new(HELLO_TEXT).render(center, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::components::Component;

    /// Scenario: HelloComponent renders a centered static text via Layout
    /// vertical and horizontal Min Length Min
    #[test]
    fn hello_component_renders_centered_static_greeting_text() {
        // @step Given an isolated HelloComponent rendered onto an 80x24 TestBackend buffer
        let mut hello = HelloComponent::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
        terminal
            .draw(|frame| {
                hello.render(frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        let buf = terminal.backend().buffer().clone();

        // @step When I scan the rendered buffer for the static greeting text
        let mut found_row: Option<u16> = None;
        let mut found_col: Option<u16> = None;
        for y in 0..buf.area.height {
            // Reconstruct the row text by concatenating cell symbols.
            let mut row = String::with_capacity(buf.area.width as usize);
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            if let Some(idx) = row.find(HELLO_TEXT) {
                found_row = Some(y);
                found_col = Some(idx as u16);
                break;
            }
        }
        let row = found_row.expect("expected to find HELLO_TEXT somewhere in the buffer");
        let col = found_col.expect("col matched alongside row");

        // @step Then the greeting text appears on a row inside the middle vertical third of the buffer
        let h = buf.area.height;
        let middle_band_lo = h / 3;
        let middle_band_hi = h - (h / 3);
        assert!(
            row >= middle_band_lo && row < middle_band_hi,
            "greeting row {row} must be inside the middle vertical third [{middle_band_lo}, {middle_band_hi})"
        );

        // @step And the greeting text is horizontally centered such that its left and right padding columns differ by at most 1
        let text_len = HELLO_TEXT.chars().count() as u16;
        let left_pad = col;
        let right_pad = buf.area.width - (col + text_len);
        let diff = left_pad.abs_diff(right_pad);
        assert!(
            diff <= 1,
            "horizontal padding must differ by at most 1 column. left={left_pad} right={right_pad} diff={diff}"
        );

        // @step And the HelloComponent never returns Consumed from handle_event
        let key = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = hello.handle_event(&key);
        assert!(
            !result.is_consumed(),
            "HelloComponent must never consume events"
        );
    }
}
