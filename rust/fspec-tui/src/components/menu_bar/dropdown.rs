//! MENU-001 — the anchored menu dropdown panel: pure geometry pass.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R7).
//!
//! [`dropdown_rect`] is the pure geometry pass: a rounded-border panel
//! anchored directly under the owning Zone A item (x = item_x,
//! y = bar_y + 1), width `max(24, widest_row + 4)`, height
//! `entries + 2` — NO title row, NO footer (GUI menu parity). The
//! panel left-clamps at the terminal's right edge and bottom-clips
//! with a dim `⋯` indicator row (painted by
//! [`super::dropdown_paint::render_menu_dropdown`]) when the terminal
//! is too short.
//!
//! The panel painter lives in [`super::dropdown_paint`]; the row
//! builder reads the [`super::items`] registry so the dropdown and the
//! 'u' help dialog can never drift.

use ratatui::layout::Rect;

use super::items::CATEGORIES;

/// The minimum panel width in columns (R7: `max(24, widest_row + 4)`).
pub const MIN_PANEL_WIDTH: u16 = 24;

/// R7: the panel's rect under the owning item. `item_x` is the Zone A
/// item rect's x, `bar_y` its row. `None` when the panel would paint
/// nowhere (no rows of clearance below the bar, or the panel is wider
/// than the terminal so it cannot fit anywhere).
pub fn dropdown_rect(area: Rect, item_x: u16, bar_y: u16, category: usize) -> Option<Rect> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let entries = CATEGORIES.get(category)?.entries;
    if entries.is_empty() {
        return None;
    }
    let widest = entries
        .iter()
        .map(|e| row_content_width(e.label, e.description))
        .max()?;
    let width = MIN_PANEL_WIDTH.max(widest as u16 + 4);
    if width > area.width {
        return None;
    }
    // Bottom clip: keep the panel inside the area (R7).
    let top = bar_y.saturating_add(1);
    let available = area.bottom().saturating_sub(top);
    let height = (entries.len() + 2).min(available as usize) as u16;
    if height < 2 {
        return None;
    }
    // Left clamp: never paint past the terminal's right edge (R7).
    let right = item_x.saturating_add(width);
    let x = if right > area.x + area.width {
        (area.x + area.width).saturating_sub(width)
    } else {
        item_x
    };
    Some(Rect {
        x,
        y: top,
        width,
        height,
    })
}

/// The natural content width of one entry row: the 4-cell key column
/// + label + the ` - ` separator + the (capped) description.
pub fn row_content_width(label: &str, description: &str) -> usize {
    super::dropdown_paint::row_content_width(label, description)
}

/// The inner content rows visible inside `panel`: `panel.height - 2`,
/// minus one more when a clipped tail needs the dim `⋯` indicator row.
pub fn visible_entry_rows(panel: Rect, entry_count: usize) -> usize {
    let content = panel.height.saturating_sub(2) as usize;
    if entry_count > content && content >= 1 {
        content - 1
    } else {
        content
    }
}

/// The first visible entry row given the cursor — auto-scrolls so the
/// cursor row is always painted (the cursor can never clip out).
pub fn scroll_window(panel: Rect, entry_count: usize, cursor: usize) -> usize {
    let rows = visible_entry_rows(panel, entry_count).max(1);
    let cursor = cursor.min(entry_count.saturating_sub(1));
    cursor.saturating_sub(rows - 1)
}

/// BUG-196 R1: hit-test for a left click on the dropdown panel. Returns
/// the index of the registry row under `(column, row)`, or `None` when
/// the click does not hit a painted entry row (border rows, the dim
/// `⋯` ellipsis row, or positions outside the panel).
///
/// The math is the SAME `scroll_window` + `visible_entry_rows` the
/// painter uses (`dropdown_paint::render_menu_dropdown`), so the
/// hit-test can never drift from the paint.
pub fn dropdown_row_at(
    panel: Rect,
    category: usize,
    cursor: usize,
    column: u16,
    row: u16,
) -> Option<usize> {
    let cat = CATEGORIES.get(category)?;
    let entries = cat.entries.len();
    // The inner content area: 1-cell border on all sides (the painter's
    // `on_edge` rule — `panel.y`, `panel.bottom()-1`, `panel.x`,
    // `panel.right()-1` are border cells).
    if row < panel.y + 1 || row >= panel.bottom().saturating_sub(1) {
        return None;
    }
    if column < panel.x + 1 || column >= panel.right().saturating_sub(1) {
        return None;
    }
    let start = scroll_window(panel, entries, cursor);
    let rows = visible_entry_rows(panel, entries);
    let content_y = (row - panel.y - 1) as usize;
    // A clipped panel paints the dim `⋯` indicator on the last content
    // row (`panel.y + 1 + rows`) — that row is not an entry row.
    if content_y >= rows {
        return None;
    }
    let entry = start + content_y;
    (entry < entries).then_some(entry)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// A Kanban panel (4 entries) in a standard 80x24 area.
    fn kanban_panel() -> Rect {
        dropdown_rect(Rect::new(0, 0, 80, 24), 1, 0, 0).expect("panel")
    }

    #[test]
    fn click_on_an_entry_row_maps_to_that_entry() {
        // @step Given the Kanban dropdown panel (4 entries, no clip)
        let panel = kanban_panel();
        // @step When a click lands on the first content row (the key column)
        let hit = dropdown_row_at(panel, 0, 0, panel.x + 1, panel.y + 1);
        // @step Then it maps to entry 0 (New Agent)
        assert_eq!(hit, Some(0), "content row 0 maps to entry 0");
        // The inner-right cell (right-2) of the 4th content row maps to entry 3.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.right() - 2, panel.y + 1 + 3),
            Some(3),
            "content row 3 maps to entry 3 (Attachments)"
        );
    }

    #[test]
    fn click_on_border_rows_maps_to_no_row() {
        let panel = kanban_panel();
        // Top border row.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, panel.y),
            None,
            "the top border row maps to no entry"
        );
        // Bottom border row.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, panel.bottom() - 1),
            None,
            "the bottom border row maps to no entry"
        );
        // Left border column.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x, panel.y + 1),
            None,
            "the left border column maps to no entry"
        );
        // Right border column.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.right() - 1, panel.y + 1),
            None,
            "the right border column maps to no entry"
        );
    }

    #[test]
    fn click_outside_the_panel_maps_to_no_row() {
        let panel = kanban_panel();
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x, panel.y - 1),
            None,
            "above the panel maps to no entry"
        );
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, panel.bottom()),
            None,
            "below the panel maps to no entry"
        );
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.right(), panel.y + 1),
            None,
            "right of the panel maps to no entry"
        );
    }

    #[test]
    fn click_on_the_ellipsis_row_maps_to_no_row() {
        // A short area: 5 rows below the bar → content = 3 rows →
        // 2 entry rows + the dim `⋯` indicator on the last content row.
        let panel = dropdown_rect(Rect::new(0, 0, 80, 6), 1, 0, 0).expect("panel");
        assert_eq!(visible_entry_rows(panel, 4), 2, "2 entry rows visible");
        let ellipsis_y = panel.y + 1 + 2;
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, ellipsis_y),
            None,
            "the dim ellipsis row maps to no entry"
        );
        // The 2 visible entry rows still map.
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, panel.y + 1),
            Some(0)
        );
        assert_eq!(
            dropdown_row_at(panel, 0, 0, panel.x + 1, panel.y + 2),
            Some(1)
        );
    }

    #[test]
    fn click_respects_the_scroll_window() {
        // Same short panel, cursor on the LAST entry (3) → the window
        // scrolls to start = 2 (rows 2..3 visible).
        let panel = dropdown_rect(Rect::new(0, 0, 80, 6), 1, 0, 0).expect("panel");
        assert_eq!(scroll_window(panel, 4, 3), 2, "window starts at entry 2");
        assert_eq!(
            dropdown_row_at(panel, 0, 3, panel.x + 1, panel.y + 1),
            Some(2),
            "the first VISIBLE content row maps to entry 2 (the scrolled window)"
        );
        assert_eq!(
            dropdown_row_at(panel, 0, 3, panel.x + 1, panel.y + 2),
            Some(3),
            "the second visible content row maps to entry 3 (the cursor row)"
        );
    }

    #[test]
    fn unknown_category_maps_to_no_row() {
        let panel = kanban_panel();
        assert_eq!(
            dropdown_row_at(panel, 99, 0, panel.x + 1, panel.y + 1),
            None,
            "an out-of-range category maps to no entry"
        );
    }
}
