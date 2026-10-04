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
