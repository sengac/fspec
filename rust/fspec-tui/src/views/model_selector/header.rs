//! RPC-338 — provider-header row rendering for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/model-selector-profile-rendering.feature
//!
//! Extracted from `rows.rs` to keep each `views/model_selector/` file under
//! the 300 LoC source-shape budget. Renders a non-selectable provider header,
//! adding a magenta 📁 icon for local-server profile sections and a red
//! `(unreachable)` marker for unreachable profiles (both adopt the selected
//! highlight style when the row is selected). Cloud headers keep the
//! pre-RPC-338 single-span rendering for snapshot parity.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::selection_style::{base_style, fill_row, token_style, NOSEL, SEL};
use crate::components::model_selector_dialog_rows::ModelSelectorRow;

/// Paint a non-selectable provider header row (no badges). RPC-351:
/// selected headers paint a solid cyan band (fg=Black) filled to the full
/// row width and prepend the `> ` selection marker BEFORE the ▼/▶ expand
/// icon (`  ` when unselected); inline 📁 / (unreachable) markers flip to
/// black on the band.
pub(super) fn render_header_row(
    area: Rect,
    buf: &mut Buffer,
    row: &ModelSelectorRow,
    is_selected: bool,
) {
    let style = base_style(is_selected);
    // RPC-351: full-width band fill so the highlight runs edge-to-edge.
    if is_selected {
        fill_row(area, buf, style);
    }
    let marker = if is_selected { SEL } else { NOSEL };
    if !(row.is_profile || row.is_unreachable) {
        // Cloud header — marker + label, no profile/unreachable markers.
        Paragraph::new(Line::from(Span::styled(
            format!("{marker}{}", row.label),
            style,
        )))
        .render(area, buf);
        return;
    }
    // Profile / unreachable header — split the arrow off so the 📁 icon sits
    // between the arrow and the label, and the (unreachable) marker after the
    // "(N models)" count.
    let (arrow, rest) = row
        .label
        .split_once(' ')
        .unwrap_or(("", row.label.as_str()));
    let mut spans: Vec<Span<'static>> = vec![Span::styled(format!("{marker}{arrow} "), style)];
    if row.is_profile {
        let folder_style = token_style(is_selected, Color::Magenta);
        spans.push(Span::styled("📁 ".to_string(), folder_style));
    }
    spans.push(Span::styled(rest.to_string(), style));
    if row.is_unreachable {
        let marker_style = token_style(is_selected, Color::Red);
        spans.push(Span::styled(" (unreachable)".to_string(), marker_style));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}
