//! RPC-016: per-column scroll viewport painter for the BoardView grid.
//!
//! Feature: spec/features/rpc016-board-viewport.feature
//!
//! Replaces RPC-014's `paint_content_rows` with a viewport-aware
//! painter that:
//!   - Reads `BoardStore::scroll_offset_for(column)` and paints `↑`
//!     on the first viewport row when offset > 0.
//!   - Paints `↓` on the last viewport row when more units exist below.
//!   - Renders `⏩ {prefix}{id}{points} ⏩` for the work unit with the
//!     largest `last_state_change_at` timestamp across the store.
//!   - Renders `🟢 ` before the work unit id when the BoardStore has
//!     an attached session for that id.
//!
//! The selected cell still wins the bg=Green/fg=Black/Bold style; the
//! ⏩/🟢 indicators sit inside the cell text so they coexist with the
//! highlight (matching the TS chalk behaviour).

use codelet_rpc_types::WorkUnitInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::store::{BoardStore, COLUMN_ORDER};
use crate::theme::Theme;
use crate::views::board::columns::pad_to_width;
use crate::views::board::grid::{column_width_at, ColumnWidths};

/// Paint every content row in the column-content area with per-column
/// viewport scrolling + indicator glyphs.
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
    let viewport_height = area.height as usize;
    let last_changed_id: Option<String> = store.last_changed_unit().map(|u| u.id.clone());
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
            paint_cell(
                buf,
                x,
                area.y + row,
                w,
                row as usize,
                viewport_height,
                col_name,
                idx,
                store,
                theme,
                last_changed_id.as_deref(),
            );
            x += w;
        }
        buf.set_string(x, area.y + row, "│", border_style);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_cell(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    w: u16,
    row: usize,
    viewport_height: usize,
    col_name: &str,
    col_idx: usize,
    store: &BoardStore,
    theme: &Theme,
    last_changed_id: Option<&str>,
) {
    let units = store.column_units(col_name);
    let scroll_offset = store.scroll_offset_for(col_name);
    let selected_idx = store.selected_index_for(col_name);
    let is_focused = store.focused_column_index() == col_idx;
    let total = units.len();
    let up_arrow_row = scroll_offset > 0;
    let down_arrow_row = scroll_offset + viewport_height < total;

    let (text, style) = if up_arrow_row && row == 0 {
        (center_glyph("↑", w), arrow_style(theme))
    } else if down_arrow_row && row + 1 == viewport_height {
        (center_glyph("↓", w), arrow_style(theme))
    } else {
        let unit_idx = scroll_offset + row;
        if let Some(unit) = units.get(unit_idx) {
            let label = build_cell_label(unit, store, last_changed_id);
            let style = if is_focused && unit_idx == selected_idx {
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
        }
    };
    let padded = pad_to_width(&text, w as usize);
    buf.set_string(x, y, padded, style);
}

fn arrow_style(theme: &Theme) -> Style {
    Style::default().fg(theme.dim)
}

fn center_glyph(glyph: &str, width: u16) -> String {
    let count = glyph.chars().count();
    if (width as usize) <= count {
        return glyph.to_string();
    }
    let pad = ((width as usize) - count) / 2;
    let mut out = String::new();
    for _ in 0..pad {
        out.push(' ');
    }
    out.push_str(glyph);
    out
}

fn build_cell_label(
    unit: &WorkUnitInfo,
    store: &BoardStore,
    last_changed_id: Option<&str>,
) -> String {
    let attached = store.session_for(&unit.id).is_some();
    let is_last_changed = last_changed_id == Some(unit.id.as_str());
    let id_with_points = match unit.estimate {
        Some(points) => format!("{} [{}]", unit.id, points),
        None => unit.id.clone(),
    };
    let prefix_attached = if attached { "🟢 " } else { "" };
    if is_last_changed {
        format!("⏩ {prefix_attached}{id_with_points} ⏩")
    } else {
        format!("{prefix_attached}{id_with_points}")
    }
}
