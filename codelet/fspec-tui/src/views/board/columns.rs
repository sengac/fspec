//! Column header + content-row painters for the BoardView grid.
//!
//! Feature: spec/features/rpc014-board-grid.feature
//! Card: RPC-014.
//!
//! Extracted from `views/board.rs` so the orchestrator stays under the
//! 300 LoC file-size invariant. These painters consume the same
//! `ColumnWidths` produced by `grid::calculate_column_widths` and read
//! state via `&BoardStore`.

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
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim)
        };
        buf.set_string(x, area.y, label, style);
        x += w;
    }
}

/// Paint every content row in the column-content area.
///
/// Row `r` of the content area maps to the work unit at index `r` of
/// each column (no scrolling — viewport scroll lands in RPC-016). The
/// `⏩` last-changed indicator and the `🟢` session-attached indicator
/// are intentionally NOT painted by this slice.
pub(crate) fn paint_content_rows(
    area: Rect,
    buf: &mut Buffer,
    widths: ColumnWidths,
    store: &BoardStore,
    theme: &Theme,
) {
    if area.height == 0 {
        return;
    }
    let border_style = Style::default().fg(theme.border);
    for row in 0..area.height {
        let mut x = area.x;
        buf.set_string(x, area.y + row, "│", border_style);
        x += 1;
        for (idx, col_name) in COLUMN_ORDER.iter().enumerate() {
            if idx > 0 {
                buf.set_string(x, area.y + row, "│", border_style);
                x += 1;
            }
            let w = column_width_at(idx, widths);
            let units = store.column_units(col_name);
            let selected_idx = store.selected_index_for(col_name);
            let is_focused = store.focused_column_index() == idx;
            let (text, style) = if let Some(unit) = units.get(row as usize) {
                let label = if let Some(points) = unit.estimate {
                    format!("{} [{}]", unit.id, points)
                } else {
                    unit.id.clone()
                };
                let style = if is_focused && (row as usize) == selected_idx {
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else if unit.work_type == "bug" {
                    Style::default().fg(Color::Red)
                } else if unit.work_type == "task" {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(theme.fg)
                };
                (label, style)
            } else {
                (String::new(), Style::default())
            };
            let padded = pad_to_width(&text, w as usize);
            buf.set_string(x, area.y + row, padded, style);
            x += w;
        }
        buf.set_string(x, area.y + row, "│", border_style);
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
