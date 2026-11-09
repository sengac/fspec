//! MENU-001 — the anchored menu dropdown panel: the stateless painter.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R7, R8).
//!
//! [`render_menu_dropdown`] paints the panel whose rect
//! [`super::dropdown::dropdown_rect`] computed: a 4-cell dim key-hint
//! column, the label (fg), a dim description (truncated with `…` when
//! the row is too wide), an inverse-video cursor row (bg Cyan /
//! fg Black / bold), and the dim `⋯` indicator row on a
//! bottom-clipped panel. The row builder reads the [`super::items`]
//! registry so the dropdown and the 'u' help dialog can never drift.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

use super::dropdown::{scroll_window, visible_entry_rows};
use super::dropdown_text::{key, truncate, KEY_COL};
use super::items::{MenuEntry, CATEGORIES};

/// The description truncation budget — a deterministic cap so panel
/// widths stay bounded regardless of how long a registry description
/// grows (R7: "truncated with an ellipsis").
pub const DESC_MAX: usize = 28;

/// The natural content width of one entry row: the 4-cell key column
/// + label + the ` - ` separator + the (capped) description.
pub fn row_content_width(label: &str, description: &str) -> usize {
    let desc = description.width().min(DESC_MAX);
    KEY_COL as usize + label.width() + 3 + desc
}

/// Paint the dropdown panel (R7). No-op when `panel` is empty.
pub fn render_menu_dropdown(
    panel: Rect,
    buf: &mut Buffer,
    category: usize,
    cursor: usize,
    theme: &Theme,
) {
    let Some(category) = CATEGORIES.get(category) else {
        return;
    };
    let entries = category.entries;
    let cursor = cursor.min(entries.len().saturating_sub(1));
    // Panel background.
    for row in panel.rows() {
        for x in row.x..row.x + row.width {
            if buf.area.contains(Position::new(x, row.y)) {
                buf[(x, row.y)].set_symbol(" ");
                buf[(x, row.y)].set_bg(Color::Black);
            }
        }
    }
    let border_style = Style::default()
        .fg(theme.border_focused)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);
    for row in panel.rows() {
        for x in row.x..row.x + row.width {
            let on_edge = x == row.x
                || x == row.right().saturating_sub(1)
                || row.y == panel.y
                || row.y == panel.bottom().saturating_sub(1);
            if !on_edge || !buf.area.contains(Position::new(x, row.y)) {
                continue;
            }
            buf[(x, row.y)].set_symbol(border_symbol(x, row.y, panel));
            buf[(x, row.y)].set_style(border_style);
        }
    }
    // Rows — painted into CONTENT rows 0..rows (relative, not the
    // absolute entry offset — the window scrolled by `start`).
    let start = scroll_window(panel, entries.len(), cursor);
    let rows = visible_entry_rows(panel, entries.len());
    for (row_idx, (offset, entry)) in entries
        .iter()
        .enumerate()
        .skip(start)
        .take(rows)
        .enumerate()
    {
        paint_dropdown_row(buf, panel, row_idx, entry, offset == cursor, theme);
    }
    // R7: the dim ellipsis row when the panel bottom-clipped.
    let rows = visible_entry_rows(panel, entries.len());
    if entries.len() > rows && rows >= 1 {
        let y = panel.y + 1 + rows as u16;
        if buf.area.contains(Position::new(panel.x + 1, y)) {
            buf[(panel.x + 1, y)].set_symbol("⋯");
            buf[(panel.x + 1, y)].set_style(Style::default().fg(theme.dim));
        }
    }
}

/// The row y for `entry_offset` inside `panel` (0-based content row).
fn row_y(panel: Rect, entry_offset: usize) -> u16 {
    panel.y + 1 + entry_offset as u16
}

/// Paints one entry row. `content_row` is the 0-based row INSIDE the
/// panel (the window may have scrolled); `selected` marks the cursor
/// row for the inverse-video highlight.
fn paint_dropdown_row(
    buf: &mut Buffer,
    panel: Rect,
    content_row: usize,
    entry: &MenuEntry,
    selected: bool,
    theme: &Theme,
) {
    let y = row_y(panel, content_row);
    if !buf.area.contains(Position::new(panel.x + 1, y)) {
        return;
    }
    // R7: the cursor row is inverse-video across its FULL inner width.
    if selected {
        let inner = Rect {
            x: panel.x + 1,
            y,
            width: panel.width.saturating_sub(2),
            height: 1,
        };
        for row in inner.rows() {
            for x in row.x..row.x + row.width {
                if buf.area.contains(Position::new(x, row.y)) {
                    buf[(x, row.y)].set_style(inverse_style());
                }
            }
        }
    }
    let key_style = Style::default().fg(theme.dim);
    let mut x = panel.x + 1;
    // 4-cell dim key-hint column (key + pad).
    x = paint_cell(buf, x, y, &key(entry.key), key_style);
    // Label (fg; inverse when selected).
    x = paint_cell(
        buf,
        x,
        y,
        entry.label,
        if selected {
            inverse_style()
        } else {
            Style::default().fg(theme.fg)
        },
    );
    x = paint_cell(buf, x, y, " - ", Style::default().fg(theme.dim));
    // Dim description, truncated with an ellipsis when the row is too wide.
    let avail = panel.right().saturating_sub(x).saturating_sub(1) as usize;
    let desc = truncate(entry.description, avail);
    paint_cell(
        buf,
        x,
        y,
        &desc,
        if selected {
            inverse_style()
        } else {
            Style::default().fg(theme.dim)
        },
    );
}

/// The R7 rounded-border glyph for the edge cell at (x, y).
fn border_symbol(x: u16, y: u16, panel: Rect) -> &'static str {
    let on_top = y == panel.y;
    let on_bottom = y == panel.bottom().saturating_sub(1);
    let on_left = x == panel.x;
    let on_right = x == panel.right().saturating_sub(1);
    match (on_top, on_bottom, on_left, on_right) {
        (true, _, true, _) => "╭",
        (true, _, _, true) => "╮",
        (_, true, true, _) => "╰",
        (_, true, _, true) => "╯",
        (true, _, _, _) => "─",
        (_, true, _, _) => "─",
        (_, _, true, _) => "│",
        _ => "│",
    }
}

/// R5/R7: the inverse-video style (bg Cyan / fg Black / bold).
fn inverse_style() -> Style {
    Style::default()
        .bg(Color::Cyan)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Paint `text` at (x, y) with one style — stops at the buffer edge.
/// Returns the x past the last painted cell.
fn paint_cell(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) -> u16 {
    let mut cx = x;
    for ch in text.chars() {
        if !buf.area.contains(Position::new(cx, y)) {
            break;
        }
        buf[(cx, y)].set_char(ch);
        buf[(cx, y)].set_style(style);
        cx = cx.saturating_add(1);
    }
    cx
}

#[cfg(test)]
mod tests;
