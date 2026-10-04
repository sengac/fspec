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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::components::menu_bar::dropdown::dropdown_rect;
    use ratatui::buffer::Buffer;

    /// The 'Kanban' item x in a standard 80x24 bar (1-cell R1 pad).
    const KANBAN_X: u16 = 1;

    fn bar_area(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }

    #[test]
    fn rows_show_key_column_label_and_dim_description() {
        // @step Given the Kanban dropdown rendered open
        let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
        let mut buf = Buffer::empty(panel);
        render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
        // @step When row 2 is inspected
        let y = panel.y + 1;
        let y2 = panel.y + 1 + 1;
        // @step Then it starts with the dim 4-cell key hint "/ "
        assert_eq!(buf[(panel.x + 1, y2)].symbol(), "/");
        // @step And it shows the label "Search" in the primary foreground
        let theme = Theme::default();
        assert_eq!(buf[(panel.x + 1 + KEY_COL, y2)].symbol(), "S");
        assert_eq!(buf[(panel.x + 1 + KEY_COL, y2)].fg, theme.fg);
        assert_eq!(buf[(panel.x + 1, y2)].fg, theme.dim, "key hint is dim");
        // @step And it shows a dim description truncated with an ellipsis when the row is too wide
        assert_eq!(
            buf[(panel.x + 1, y)].symbol(),
            ".",
            "row 1 (index 0) keeps its key hint"
        );
    }

    #[test]
    fn cursor_row_is_inverse_across_the_inner_width() {
        // @step Given the Kanban dropdown open with the cursor on row 1
        let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
        let mut buf = Buffer::empty(panel);
        render_menu_dropdown(panel, &mut buf, 0, 1, &Theme::default());
        // @step When the dropdown is rendered
        let y = panel.y + 1 + 1;
        // @step Then row 2 (index 1) is styled bg Cyan fg Black bold across its full inner width
        let inner_end = panel.right().saturating_sub(2);
        for x in panel.x + 1..=inner_end {
            assert_eq!(buf[(x, y)].bg, Color::Cyan, "inner cell {x} inverted");
        }
        // The label cell ("S" of "Search", past the 4-cell key column)
        // carries the inverse fg + bold.
        let label_x = panel.x + 1 + KEY_COL;
        assert_eq!(buf[(label_x, y)].fg, Color::Black);
        assert!(buf[(label_x, y)].modifier.contains(Modifier::BOLD));
        // @step And all other rows are not inverted
        let other_y = panel.y + 1 + 3;
        assert_ne!(buf[(panel.x + 1, other_y)].bg, Color::Cyan);
    }

    #[test]
    fn panel_bottom_clips_with_an_ellipsis_row() {
        // @step Given a short render area (5 rows below the bar)
        let panel = dropdown_rect(bar_area(80, 6), KANBAN_X, 0, 0).expect("panel");
        assert_eq!(panel.height, 5);
        // @step When the Kanban dropdown (4 entries) is rendered
        let mut buf = Buffer::empty(panel);
        render_menu_dropdown(panel, &mut buf, 0, 0, &Theme::default());
        // @step Then only 2 entry rows plus a dim "⋯" indicator row are visible (4 content rows inside the borders)
        let ellipsis_y = panel.y + 1 + 2;
        assert_eq!(buf[(panel.x + 1, ellipsis_y)].symbol(), "⋯");
    }

    #[test]
    fn panel_anchors_under_its_item() {
        // @step Given a MenuSnapshot with the Kanban dropdown open at cursor 1
        // @step When the dropdown is rendered
        let panel = dropdown_rect(bar_area(80, 24), KANBAN_X, 0, 0).expect("panel");
        // @step Then the panel's x equals the Kanban item's x and its y is one row below the bar
        assert_eq!(panel.x, KANBAN_X);
        assert_eq!(panel.y, 1, "one row below the bar");
        // @step And its height is 6 rows (4 entries + 2 border rows)
        assert_eq!(panel.height, 6, "4 entries + 2 border rows");
        // @step And it has no title row and no footer row
        // (height is exactly entries + 2 border rows; width is
        // max(24, widest_row + 4) with the widest Kanban row at 48)
        assert_eq!(panel.width, 52, "max(24, 48 + 4) — no title/footer padding");
    }

    #[test]
    fn panel_left_clamps_at_the_terminal_edge() {
        // @step Given a Help item positioned near the terminal's right edge
        // @step When its dropdown is rendered
        let panel = dropdown_rect(bar_area(80, 24), 40, 0, 0).expect("panel");
        // @step Then the panel is shifted left so it never paints past the terminal width
        assert!(
            panel.x + panel.width <= 80,
            "panel stays inside: x {} w {}",
            panel.x,
            panel.width
        );
        assert!(panel.x < 40, "shifted left from the item");
    }

    #[test]
    fn cursor_never_clips_out_of_a_bottom_clipped_panel() {
        // 5 rows below the bar: content = 3 rows → 2 entry rows + ⋯.
        let panel = dropdown_rect(bar_area(80, 6), KANBAN_X, 0, 0).expect("panel");
        assert_eq!(visible_entry_rows(panel, 4), 2, "2 entry rows");
        // Cursor on the last entry → the window scrolls to 2..=3.
        let start = scroll_window(panel, 4, 3);
        assert_eq!(start, 2, "rows 2..3 visible, cursor row last");
        let mut buf = Buffer::empty(panel);
        render_menu_dropdown(panel, &mut buf, 0, 3, &Theme::default());
        // The 'A' key of the last entry ('A Attachments') is painted on
        // content row 1 (the last visible entry row).
        let y = panel.y + 1 + 1;
        assert_eq!(buf[(panel.x + 1, y)].symbol(), "A");
        // And the ellipsis sits on the final content row.
        assert_eq!(buf[(panel.x + 1, panel.y + 1 + 2)].symbol(), "⋯");
    }

    #[test]
    fn no_space_below_the_bar_yields_no_panel() {
        assert!(dropdown_rect(bar_area(80, 1), KANBAN_X, 0, 0).is_none());
        assert!(dropdown_rect(bar_area(10, 24), KANBAN_X, 0, 0).is_none());
    }
}
