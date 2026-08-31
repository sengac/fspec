//! MUX-004 — row builders + footer for the MuxConfigDialog.
//!
//! Feature: spec/features/mux-config-dialog.feature
//!
//! Extracted from `mux_config_dialog.rs` to keep both files under the
//! 300-LoC ceiling. One row per editable field (Enabled, Orientation,
//! then one per configured pane in grid order) plus the fixed footer
//! hint string.

use crate::views::multiplex::{MuxConfig, MuxOrientation, MuxPaneKind};

use super::dialog_theme::DialogRow;
use super::dialog_theme_rows::label_description_row;

/// The fixed MuxConfigDialog footer (R4 keybindings).
pub const MUX_CONFIG_DIALOG_FOOTER: &str =
    "↑↓ Field · ←→ Value · A Add · ⌫ Remove · S Save · Enter Apply · Esc Cancel";

/// The dialog title.
pub const MUX_CONFIG_DIALOG_TITLE: &str = "Mux Layout";

/// Display label for a pane kind (the R3/R4 cycle names).
pub fn pane_kind_label(kind: MuxPaneKind) -> &'static str {
    match kind {
        MuxPaneKind::Board => "Board",
        MuxPaneKind::Agent => "Agent",
        MuxPaneKind::ChangedFiles => "Files",
        MuxPaneKind::Checkpoints => "Checkpoints",
    }
}

/// Orientation display label.
pub fn orientation_label(orientation: MuxOrientation) -> &'static str {
    match orientation {
        MuxOrientation::Horizontal => "Horizontal",
        MuxOrientation::Vertical => "Vertical",
    }
}

/// Build the dialog body rows (R3 order: Enabled, Orientation, then one
/// row per configured pane in grid order). `cursor` is the highlighted
/// row index (0 = Enabled, 1 = Orientation, 2+i = Pane i+1).
pub fn build_rows(draft: &MuxConfig, cursor: usize) -> Vec<DialogRow> {
    let mut rows: Vec<DialogRow> = vec![
        label_description_row(
            "Enabled",
            if draft.enabled { "On" } else { "Off" },
            cursor == 0,
        ),
        label_description_row(
            "Orientation",
            orientation_label(draft.orientation),
            cursor == 1,
        ),
    ];
    for (i, kind) in draft.panes.iter().enumerate() {
        rows.push(label_description_row(
            &format!("Pane {}", i + 1),
            pane_kind_label(*kind),
            cursor == 2 + i,
        ));
    }
    rows
}
