//! Column header painter for the BoardView grid.
//!
//! Feature: spec/features/rpc014-board-grid.feature
//! Card: RPC-014 (header painter), RPC-016 (viewport painter moved to
//!       `views/board/viewport.rs`).
//!
//! Extracted from `views/board.rs` so the orchestrator stays under the
//! 300 LoC file-size invariant. Per-row content rendering plus the
//! `↑`/`↓` scroll arrows and `⏩`/`🟢` indicators live in the sibling
//! `viewport` module after RPC-016.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::store::{BoardStore, COLUMN_ORDER};
use crate::theme::Theme;
use crate::views::board::grid::{column_width_at, ColumnWidths};

/// Paint the row of seven column headers (each padded to its
/// per-column width). The focused column is rendered cyan + bold;
/// the others use the theme's dim foreground.
pub(crate) fn paint_column_headers(
    area: Rect,
    buf: &mut Buffer,
    widths: ColumnWidths,
    store: &BoardStore,
    theme: &Theme,
) {
    let mut x = area.x + 1; // skip left │
    for (idx, col) in COLUMN_ORDER.iter().enumerate() {
        if idx > 0 {
            buf.set_string(x, area.y, "│", Style::default().fg(theme.border));
            x += 1;
        }
        let w = column_width_at(idx, widths);
        let label = pad_to_width(&col.to_uppercase(), w as usize);
        let style = if store.focused_column_index() == idx {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim)
        };
        buf.set_string(x, area.y, label, style);
        x += w;
    }
}

/// Pad `s` with trailing spaces (or truncate it) so the resulting
/// string has exactly `width` user-perceived characters.
pub(crate) fn pad_to_width(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if count >= width {
        let mut out = String::with_capacity(width);
        for ch in s.chars().take(width) {
            out.push(ch);
        }
        return out;
    }
    let mut out = String::with_capacity(width);
    out.push_str(s);
    for _ in count..width {
        out.push(' ');
    }
    out
}
