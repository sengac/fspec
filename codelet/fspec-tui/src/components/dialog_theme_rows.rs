//! RPC-027 — Helper row builders for the canonical dialog theme.
//!
//! Feature: spec/features/rpc027-dialog-theme.feature
//!
//! Extracted from `dialog_theme.rs` to keep both files under the
//! 300-LoC ceiling required by RPC-027 rule [11].

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use super::dialog_theme::{DialogRow, MARKER_SELECTED, MARKER_UNSELECTED};

/// Convenience builder for a "marker + label - description" row used
/// by `ThinkingLevelDialog` and `ModelSelectorDialog`. The description
/// is dimmed when not selected (matches `dimColor={!isSelected}` in
/// `ThinkingLevelDialog.tsx`).
pub fn label_description_row(label: &str, description: &str, selected: bool) -> DialogRow {
    let marker = if selected {
        MARKER_SELECTED
    } else {
        MARKER_UNSELECTED
    };
    let mut spans: Vec<Span<'static>> = vec![Span::raw(marker.to_string())];
    spans.push(Span::raw(label.to_string()));
    if !description.is_empty() {
        spans.push(Span::raw(" - ".to_string()));
        let desc_style = if selected {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(description.to_string(), desc_style));
    }
    DialogRow {
        spans,
        selectable: true,
        selected,
    }
}
