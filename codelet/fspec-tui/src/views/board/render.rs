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

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;

use crate::store::{BoardStore, COLUMN_ORDER};

use super::grid::{build_border_row, column_width_at, slice_column_rects, SeparatorType};
use super::{
    borders, calculate_column_widths, details_strip, footer, header, paint_column_headers,
    paint_content_rows, BoardView,
};

/// Render the rich BoardView against the supplied store.
pub(super) fn render_with_store(
    view: &BoardView,
    area: Rect,
    buf: &mut Buffer,
    store: &BoardStore,
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
    header::paint(borders::inner_rect(split[1]), buf, store, &view.theme);
    borders::paint_border_string(
        split[2],
        buf,
        &build_border_row(widths, "├", "┤", SeparatorType::Plain),
        border_style,
    );
    borders::paint_side_borders(split[3], buf, border_style);
    details_strip::render(
        borders::inner_rect(split[3]),
        buf,
        store.selected_work_unit(),
    );
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
}
