//! RPC-337 — body/row/scrollbar rendering for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Extracted from `rows.rs` (PROV-107) so each file stays under the
//! 300-LoC ceiling. Owns the windowed list paint with a dedicated
//! scrollbar column (PROV-104) and per-token badge colouring; the row
//! projection + badge helpers remain in `rows.rs`.

use super::super::header::render_header_row;
use super::super::selection_style::{base_style, fill_row, token_style, NOSEL, SEL};
use super::{badge_token_style, EMPTY_PLACEHOLDER, LEGEND, LOADING_PLACEHOLDER};
use crate::components::model_selector_dialog_rows::ModelSelectorRow;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Paint the body region: a distinct loading indicator while providers
/// have not finished loading, an explicit no-models empty state once
/// loading completes with nothing to show, otherwise a windowed list with
/// a proportional scrollbar in a DEDICATED column beside the list (TS
/// parity), coloured badges, a green `(current)` marker on the active-session
/// model, and the legend on the bottom row. The full visible window slices
/// content rows — the scrollbar steals NO content row, so the selected row
/// at a viewport edge is always painted.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_body(
    area: Rect,
    buf: &mut Buffer,
    rows: &[ModelSelectorRow],
    loaded: bool,
    selected_index: usize,
    scroll_offset: usize,
    current_model_id: Option<&str>,
) {
    if area.height == 0 {
        return;
    }
    // Reserve the bottom row for the legend.
    let legend_y = area.y + area.height - 1;
    let legend_row = Rect {
        x: area.x,
        y: legend_y,
        width: area.width,
        height: 1,
    };
    Paragraph::new(Span::styled(
        LEGEND,
        Style::default().add_modifier(Modifier::DIM),
    ))
    .render(legend_row, buf);

    let list_height = area.height.saturating_sub(1);
    if list_height == 0 {
        return;
    }

    if rows.is_empty() {
        // PROV-104 rules [8]/[9]/[10]: a not-yet-loaded list shows a distinct
        // loading indicator; a loaded-but-empty list shows an explicit
        // no-models empty state. The two must never be indistinguishable.
        let message = if loaded {
            EMPTY_PLACEHOLDER
        } else {
            LOADING_PLACEHOLDER
        };
        let mid_y = area.y.saturating_add(list_height / 2);
        let row = Rect {
            x: area.x,
            y: mid_y,
            width: area.width,
            height: 1,
        };
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }

    let visible_rows = list_height as usize;
    let total = rows.len();
    let so = scroll_offset.min(total.saturating_sub(1));
    let end = (so + visible_rows).min(total);
    // Scrollbar column (drawn only when the list overflows the viewport).
    // TS parity (ModelSelectorView.tsx): the scroll indicator lives in a
    // DEDICATED column beside the list and steals ZERO content rows, so the
    // full visible window always paints content and the selected row at an
    // edge is always visible.
    let overflow = total > visible_rows;
    let list_width = if overflow {
        area.width.saturating_sub(1)
    } else {
        area.width
    };

    for (rel, abs_i) in (so..end).enumerate() {
        let y = area.y + rel as u16;
        let row_area = Rect {
            x: area.x,
            y,
            width: list_width,
            height: 1,
        };
        render_row(
            row_area,
            buf,
            &rows[abs_i],
            abs_i == selected_index,
            current_model_id,
        );
    }

    if overflow {
        crate::components::list_scrollbar::render_list_scrollbar(
            Rect {
                x: area.x + list_width,
                y: area.y,
                width: 1,
                height: list_height,
            },
            buf,
            so,
            visible_rows,
            total,
        );
    }
}

/// Paint one row: marker + label + coloured badges + optional green
/// `(current)` marker. Selected rows paint a solid cyan band (fg=Black)
/// filled to the full row width, with a `> ` arrow; every inline token
/// flips to black (RPC-351).
fn render_row(
    area: Rect,
    buf: &mut Buffer,
    row: &ModelSelectorRow,
    is_selected: bool,
    current_model_id: Option<&str>,
) {
    if !row.selectable {
        // RPC-338: provider header rendering (incl. 📁 / unreachable markers)
        // lives in `header.rs` to keep this file under the source-shape budget.
        render_header_row(area, buf, row, is_selected);
        return;
    }
    let base = base_style(is_selected);
    // RPC-351: pre-fill the full row width with the band so the cyan
    // highlight runs edge-to-edge (mirrors provider_settings/row_render.rs).
    if is_selected {
        fill_row(area, buf, base);
    }
    // Model rows use the deeper indent + `> ` arrow (TS `  > ` / `    `).
    let marker = if is_selected { SEL } else { NOSEL };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(format!("  {marker}"), base),
        Span::styled(row.label.clone(), base),
    ];
    // Badges — flip to black on the selected band; otherwise coloured + DIM.
    for token in row.badges.split_whitespace() {
        let style = if is_selected {
            token_style(true, Color::Black)
        } else {
            badge_token_style(token).add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(format!(" {token}"), style));
    }
    if current_model_id.is_some_and(|c| super::super::model_id::model_ids_match(c, &row.model_id)) {
        let style = token_style(is_selected, Color::Green);
        spans.push(Span::styled(" (current)".to_string(), style));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}
