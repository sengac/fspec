//! ThinkingLevelDialog rows (RPC-027 dialog_theme).
//! MODEL-011: a model with no reasoning control shows the reason above
//! Model default; an open number field is the pinned query row, with its
//! refusal under the rows.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

use super::{ThinkingLevelDialog, MODEL_DEFAULT_DESCRIPTION, MODEL_DEFAULT_LABEL};
use crate::components::dialog_theme::DialogRow;
use crate::components::dialog_theme_rows::{label_description_default_row, label_description_row};

impl ThinkingLevelDialog {
    /// The painted rows (the no-control reason first, the level rows, then
    /// an open field's refusal), the number field's pinned query row, and
    /// how many rows are clickable.
    pub(super) fn dialog_rows(&self) -> (Vec<DialogRow>, Option<String>, usize) {
        let mut rows = Vec::new();
        if let Some(reason) = &self.no_control {
            rows.push(label_description_row(reason, "", false));
        }
        for (i, choice) in self.choices.iter().enumerate() {
            rows.push(label_description_default_row(
                &choice.label(),
                &self.description(choice),
                i == self.selected_index,
                Some(i) == self.default_index,
            ));
        }
        let default_row = self.choices.len();
        rows.push(label_description_default_row(
            MODEL_DEFAULT_LABEL,
            MODEL_DEFAULT_DESCRIPTION,
            default_row == self.selected_index,
            Some(default_row) == self.default_index,
        ));
        let clickable = rows.len();
        let query = self.field.as_ref().map(|field| {
            if let Some(refusal) = &field.refusal {
                rows.push(label_description_row(refusal, "", false));
            }
            format!("{}: {}", self.choices[field.row].label(), field.text)
        });
        (rows, query, clickable)
    }
}
