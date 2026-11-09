//! BOARD-024 — chooser rendering (canonical dialog conventions).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Span;

use super::sanitize::sanitize_line;
use super::BoardCardDialog;
use crate::components::dialog_theme::{
    render_dialog, Accent, DialogRow, FspecDialog, MARKER_SELECTED, MARKER_UNSELECTED,
};

/// Chooser labels; index 0 is the default agent entry.
pub(super) const CHOICES: [&str; 2] = ["Open in agent", "Show full details"];
const FOOTER: &str = "↑ ↓ Select │ Enter Choose │ Esc Cancel";

impl BoardCardDialog {
    pub(super) fn render_chooser(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = CHOICES
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let selected = index == self.chooser_selected;
                let marker = if selected {
                    MARKER_SELECTED
                } else {
                    MARKER_UNSELECTED
                };
                DialogRow {
                    spans: vec![Span::raw(format!("{marker}{label}"))],
                    selectable: true,
                    selected,
                }
            })
            .collect();
        let title = format!("Card {}", sanitize_line(&self.card_id));
        let dialog = FspecDialog {
            accent: Accent::Yellow,
            title: &title,
            rows,
            footer: FOOTER,
            min_width: 44,
            query_row: None,
        };
        render_dialog(area, buf, &dialog);
    }
}
