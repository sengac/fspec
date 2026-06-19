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
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::model_selector_dialog_rows::ModelSelectorRow;

/// Paint a non-selectable provider header row (no marker, no badges).
pub(super) fn render_header_row(
    area: Rect,
    buf: &mut Buffer,
    row: &ModelSelectorRow,
    is_selected: bool,
) {
    let style = if is_selected {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default()
    };
    if !(row.is_profile || row.is_unreachable) {
        // Cloud header — unchanged single-span rendering (snapshot parity).
        Paragraph::new(Line::from(Span::styled(format!(" {}", row.label), style)))
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
    let mut spans: Vec<Span<'static>> = vec![Span::styled(format!(" {arrow} "), style)];
    if row.is_profile {
        let folder_style = selected_or(is_selected, style, Color::Magenta);
        spans.push(Span::styled("📁 ".to_string(), folder_style));
    }
    spans.push(Span::styled(rest.to_string(), style));
    if row.is_unreachable {
        let marker_style = selected_or(is_selected, style, Color::Red);
        spans.push(Span::styled(" (unreachable)".to_string(), marker_style));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}

/// When selected, markers adopt the base highlight `style`; otherwise they use
/// their accent colour (magenta 📁 / red unreachable), matching TS.
fn selected_or(is_selected: bool, base: Style, accent: Color) -> Style {
    if is_selected {
        base
    } else {
        Style::default().fg(accent)
    }
}
