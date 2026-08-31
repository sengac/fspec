//! MUX-001 — mux presets: the default grid + pane-count expansion.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Default preset (R1 / rule [10]): horizontal orientation,
//! Board | Agent, 50/50 split, agent pane focused.

use super::{MuxConfig, MuxOrientation, MuxPaneKind};

/// The canonical pane-kind order used to expand `/mux <n>` pane counts.
pub const DEFAULT_PANES: [MuxPaneKind; 4] = [
    MuxPaneKind::Board,
    MuxPaneKind::Agent,
    MuxPaneKind::ChangedFiles,
    MuxPaneKind::Checkpoints,
];

/// The default preset: horizontal Board | Agent at 50/50, agent pane
/// focused, mux enabled.
pub fn default_config() -> MuxConfig {
    MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50],
        panes: vec![MuxPaneKind::Board, MuxPaneKind::Agent],
        focused_pane: 1, // agent pane focused (default preset, R1)
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn default_preset_shape() {
        let cfg = default_config();
        assert!(cfg.enabled);
        assert_eq!(cfg.orientation, MuxOrientation::Horizontal);
        assert_eq!(cfg.panes, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
        assert_eq!(cfg.splits, vec![50]);
        assert_eq!(cfg.focused_pane, 1);
    }
}
