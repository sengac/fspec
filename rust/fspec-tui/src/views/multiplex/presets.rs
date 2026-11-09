//! MUX-001 — mux presets: the default layout.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Default preset (R1 / rule [10], MUX-010 layout string): `board, agent`
//! (two equal columns), agent pane focused.

use super::{MuxConfig, MuxLayout, MuxNode, MuxPaneKind};

/// The default layout `board, agent`.
pub fn default_layout() -> MuxLayout {
    MuxLayout::with_first(
        MuxNode::pane(MuxPaneKind::Board),
        vec![MuxNode::pane(MuxPaneKind::Agent)],
    )
}

/// The default preset: `board, agent`, agent pane focused, mux enabled.
pub fn default_config() -> MuxConfig {
    MuxConfig {
        layout: default_layout(),
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
        assert_eq!(cfg.layout.to_string(), "board, agent");
        assert_eq!(cfg.focused_pane, 1);
    }
}
