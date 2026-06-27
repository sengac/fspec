//! RPC-367 — shared pane chrome for the diff-style views.
//!
//! Feature: spec/features/rust-tui-pane-borders-changed-files.feature
//! Feature: spec/features/rust-tui-pane-borders-checkpoints.feature
//!
//! Lifts the previously-duplicated `pane_header` out of
//! `views/changed_files/render.rs` and `views/checkpoints/render.rs` into a
//! single shared helper (DRY) and extends it to paint a `─` underline rule
//! beneath the heading. Also adds `render_vertical_divider`, which paints a
//! `│` divider down a reserved 1-column gutter between horizontally-split
//! panes. Both dividers/rules use the default terminal colour (no explicit
//! colour set), matching the TypeScript reference.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Paint a focus-aware 1-row pane header followed by a 1-row `─` underline
/// rule, and return the content Rect that sits BELOW the underline.
///
/// Layout reserves three vertical bands: `[heading(1), underline(1),
/// content(min)]`. The heading highlights with a green band when `focused`;
/// the underline rule always uses the default terminal colour. The returned
/// content Rect starts on the row directly below the underline so list/diff
/// rows never overdraw the heading or its rule.
pub fn pane_header(area: Rect, buf: &mut Buffer, label: &str, focused: bool) -> Rect {
    if area.height == 0 {
        return area;
    }
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    Paragraph::new(Line::from(Span::styled(label.to_string(), style))).render(split[0], buf);
    render_heading_underline(split[1], buf);
    split[2]
}

/// Paint a full-width `─` underline rule across `area` using the default
/// terminal colour. A zero-height/width area paints nothing.
fn render_heading_underline(area: Rect, buf: &mut Buffer) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let rule: String = "─".repeat(area.width as usize);
    let row = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    Paragraph::new(Line::from(Span::styled(
        rule,
        Style::default().fg(Color::Reset),
    )))
    .render(row, buf);
}

/// Paint a `│` vertical divider down the reserved 1-column `gutter` Rect
/// using the default terminal colour. A zero-size gutter paints nothing.
pub fn render_vertical_divider(gutter: Rect, buf: &mut Buffer) {
    if gutter.width == 0 || gutter.height == 0 {
        return;
    }
    for i in 0..gutter.height {
        let cell = Rect {
            x: gutter.x,
            y: gutter.y + i,
            width: 1,
            height: 1,
        };
        Paragraph::new(Span::styled("│", Style::default().fg(Color::Reset))).render(cell, buf);
    }
}
