//! RPC-374: BoardView `render_with_store` extracted to keep
//! `views/board.rs` under the 300 LoC ceiling.
//!
//! Feature files:
//!   - spec/features/rpc014-board-grid.feature
//!   - spec/features/rust-board-open-attachment.feature
//!
//! Composes the box-drawing topology row-by-row: top border, 4-row header
//! strip (RPC-015 logo + checkpoints + keybindings), ├──┤ plain separator,
//! 5-row details strip, ├┬┤ separator, column header row, ├┼┤ separator,
//! content rows, ├┴┤ separator, RPC-013 footer string, bottom border. It
//! also caches the geometry the keyboard + mouse handlers read back.

use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;

use crate::components::menu_bar::{dropdown_rect, paint_menu_bar, render_menu_dropdown};
use crate::store::{AgentViewStore, BoardStore, COLUMN_ORDER};

use super::grid::{build_border_row, column_width_at, slice_column_rects, SeparatorType};
use super::menu_mouse::MenuBarGeometry;
use super::{
    borders, calculate_column_widths, details_strip, footer, header, menu_snapshot,
    paint_column_headers, paint_content_rows, BoardView,
};

/// The wall clock in ms since the epoch — drives the Running chip's
/// braille frame (MENU-002 R4).
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Render the rich BoardView against the supplied store.
/// `menu_suppressed` (MENU-004 R-SUPPRESS): when true the header's
/// exposed row 3 stays BLANK — the owning surface paints the bar
/// (the mux top row) and this pane's geometry cache is cleared so the
/// pane's mouse arms stay inert.
pub(super) fn render_with_store(
    view: &BoardView,
    area: Rect,
    buf: &mut Buffer,
    store: &BoardStore,
    agent_store: &AgentViewStore,
    menu_suppressed: bool,
) {
    if area.width < 4 || area.height < 17 {
        return;
    }
    let widths = calculate_column_widths(area.width);
    let inner_width: u16 = (0..COLUMN_ORDER.len() as u16)
        .map(|i| column_width_at(i as usize, widths))
        .sum::<u16>()
        + (COLUMN_ORDER.len() as u16 - 1);
    if inner_width + 2 > area.width {
        return;
    }
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top border
            Constraint::Length(4), // RPC-015 header strip
            Constraint::Length(1), // ├──┤ plain separator (RPC-015)
            Constraint::Length(5), // details strip (RPC-014)
            Constraint::Length(1), // ├┬┤ separator
            Constraint::Length(1), // column header
            Constraint::Length(1), // ├┼┤ separator
            Constraint::Min(0),    // content
            Constraint::Length(1), // ├┴┤ separator
            Constraint::Length(1), // footer
            Constraint::Length(1), // bottom border
        ])
        .split(area);
    let border_style = Style::default().fg(view.theme.border);

    borders::paint_border_string(
        split[0],
        buf,
        &build_border_row(widths, "┌", "┐", SeparatorType::Plain),
        border_style,
    );
    borders::paint_side_borders(split[1], buf, border_style);
    let rows = header::paint(borders::inner_rect(split[1]), buf, store, &view.theme);
    // MENU-002: the 2-zone menu bar replaces the 'u Actions' chord in
    // the header's exposed row 3. Build the per-frame snapshot (R9:
    // chips from the painted session list), paint the bar, and cache
    // the geometry the mouse arms hit-test.
    let snapshot =
        menu_snapshot::build_snapshot(agent_store, store.menu_focus(), store.open_menu(), now_ms());
    let bar_rect = rows.bar_row;
    let mut open_panel = None;
    // MENU-004 R-SUPPRESS: the mux board pane paints NO per-pane bar —
    // row 3 stays blank and the geometry cache is cleared (the pane's
    // mouse arms stay inert; the bar's geometry lives on the mux layer).
    if !menu_suppressed {
        if let Some(layout) = paint_menu_bar(bar_rect, buf, &snapshot, &view.theme) {
            if let Some((cat, _)) = store.open_menu() {
                let item_x = layout
                    .item_rects
                    .get(cat)
                    .map(|r| r.x)
                    .unwrap_or(bar_rect.x);
                open_panel = dropdown_rect(area, item_x, bar_rect.y, cat);
            }
            view.cache_menu_geometry(Some(MenuBarGeometry {
                bar_y: bar_rect.y,
                layout,
                open_panel,
            }));
        }
    } else {
        view.cache_menu_geometry(None);
    }
    borders::paint_border_string(
        split[2],
        buf,
        &build_border_row(widths, "├", "┤", SeparatorType::Plain),
        border_style,
    );
    borders::paint_side_borders(split[3], buf, border_style);
    let details_area = borders::inner_rect(split[3]);
    details_strip::render(details_area, buf, store.selected_work_unit());
    // COPY-009: cache the strip inner rect for mouse hit-testing, clear a
    // stale selection when the selected unit changed, then overlay the
    // REVERSED highlight over the selected strip rows (never the borders).
    view.last_details_area.set(Some(details_area));
    view.sync_details_selection(store.selected_work_unit());
    view.paint_details_highlight(details_area, buf);
    borders::paint_border_string(
        split[4],
        buf,
        &build_border_row(widths, "├", "┤", SeparatorType::Top),
        border_style,
    );
    borders::paint_side_borders(split[5], buf, border_style);
    paint_column_headers(split[5], buf, widths, store, &view.theme);
    // RPC-023: cache per-column header rects for click-to-focus.
    view.last_column_header_areas
        .set(Some(slice_column_rects(split[5], widths)));
    borders::paint_border_string(
        split[6],
        buf,
        &build_border_row(widths, "├", "┤", SeparatorType::Cross),
        border_style,
    );
    // RPC-016: record the viewport height the painter is about to observe
    // so handle_event can emit ScrollFocusedColumnUp/Down with the right step.
    view.last_viewport_height.set(split[7].height);
    // RPC-023: cache the content rect + per-column content rects for
    // wheel + click hit-testing.
    view.last_content_area.set(Some(split[7]));
    view.last_column_content_areas
        .set(Some(slice_column_rects(split[7], widths)));
    paint_content_rows(split[7], buf, widths, store, &view.theme);
    borders::paint_border_string(
        split[8],
        buf,
        &build_border_row(widths, "├", "┤", SeparatorType::Bottom),
        border_style,
    );
    borders::paint_side_borders(split[9], buf, border_style);
    footer::render(borders::inner_rect(split[9]), buf, &view.theme);
    borders::paint_border_string(
        split[10],
        buf,
        &build_border_row(widths, "└", "┘", SeparatorType::Plain),
        border_style,
    );
    // MENU-002: the open dropdown is an OVERLAY — painted last so it
    // sits on top of the details strip + column content it anchors
    // over (the panel's own bg fill clears what was beneath).
    // MENU-004 R-SUPPRESS: a suppressed pane owns no panel (its bar
    // never painted — `open_panel` stays `None`), so the overlay is
    // skipped automatically.
    if !menu_suppressed {
        if let Some((cat, cursor)) = store.open_menu() {
            if let Some(panel) = open_panel {
                render_menu_dropdown(panel, buf, cat, cursor, &view.theme);
            }
        }
    }
}
