//! BUG-159 — Painters extracted from `dialog_theme.rs` so that file
//! stays under the 300-LoC ceiling required by RPC-027 rule [11].
//!
//! Feature: spec/features/board-search-dialog-pinned-query-row-and-fixed-frame.feature

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::dialog_theme_rows::{line_width, paint_left_aligned, paint_text};

/// Paint the dim centered footer lines pinned to the bottom of the body
/// block. Extracted from `dialog_theme::render_dialog_at`.
pub(super) fn paint_footer(
    buf: &mut Buffer,
    body: Rect,
    footer_h: u16,
    footer: &str,
    bg_style: Style,
) {
    if footer_h == 0 || body.height < footer_h {
        return;
    }
    let footer_y = body.y + body.height - footer_h;
    for (i, line) in footer.lines().enumerate() {
        let r = Rect {
            x: body.x,
            y: footer_y + i as u16,
            width: body.width,
            height: 1,
        };
        let dim_style = Style::default()
            .add_modifier(Modifier::DIM)
            .bg(Color::Black);
        // Center horizontally.
        let line_len = line_width(line);
        let offset = if r.width > line_len {
            (r.width - line_len) / 2
        } else {
            0
        };
        for x in r.x..r.x + r.width {
            buf[(x, r.y)].set_style(bg_style);
            buf[(x, r.y)].set_symbol(" ");
        }
        paint_text(
            buf,
            r.x + offset,
            r.y,
            r.width.saturating_sub(offset),
            line,
            dim_style,
        );
    }
}

/// BUG-159: paint the pinned query row (the dialog's input line) at
/// `y` in the dialog accent color with a trailing block cursor. The
/// body rows start one row below it.
pub(super) fn paint_query_row(
    buf: &mut Buffer,
    body: Rect,
    y: u16,
    query: &str,
    accent: Color,
    bg_style: Style,
) {
    let query_style = Style::default()
        .fg(accent)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let spans: Vec<Span<'static>> = vec![
        Span::styled("▸ ".to_string(), query_style),
        Span::styled(query.to_string(), query_style),
        Span::styled("▏".to_string(), query_style),
    ];
    paint_left_aligned(
        buf,
        Rect {
            x: body.x,
            y,
            width: body.width,
            height: 1,
        },
        &spans,
        bg_style,
    );
}
