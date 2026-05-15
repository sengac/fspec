//! Theme palette (RPC-008 rules [10] [16]).
//!
//! Per RPC-002 doc 07 §6: a tiny color palette every layer reads via
//! `Arc<Theme>` so the whole app shares one source of truth for fg/bg
//! and accent colors. Only the dark variant is implemented in this card
//! — a light variant is explicitly deferred to its own work unit per
//! architecture note [10].
//!
//! The struct is deliberately small (10 fields) and uses only
//! `ratatui::style::Color` so it can be cloned cheaply and held inside
//! an `Arc<Theme>` from `App`.
//!
//! Future cards (RPC-009 list view, RPC-002 Slices 03+) extend this by
//! adding light/high-contrast variants and additional accent slots —
//! NOT by reshaping the existing fields.
//!
//! Feature: spec/features/fspec-tui-app-shell.feature (Background — App
//! holds `Arc<Theme>`).

use ratatui::style::Color;

/// Color palette shared by every layer in the App. Only `Theme::default()`
/// (dark variant) is implemented in this card — a light variant arrives
/// in its own work unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Primary foreground color.
    pub fg: Color,
    /// Primary background color.
    pub bg: Color,
    /// Default border color (unfocused).
    pub border: Color,
    /// Border color when a layer holds focus.
    pub border_focused: Color,
    /// Background of a selected row / cell.
    pub selection_bg: Color,
    /// Foreground of a selected row / cell.
    pub selection_fg: Color,
    /// Dimmed / disabled / placeholder text.
    pub dim: Color,
    /// Error accent.
    pub error: Color,
    /// Warning accent.
    pub warning: Color,
    /// Success accent.
    pub success: Color,
}

impl Default for Theme {
    /// Dark variant per RPC-002 doc 07 §6.
    fn default() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Black,
            // Borders match TS: <Text> with no color → terminal default fg.
            // Color::Reset uses the terminal's own foreground so the box
            // grid stays clearly visible (DarkGray on Black collapsed to
            // near-invisible on most modern terminals).
            border: Color::Reset,
            border_focused: Color::Cyan,
            selection_bg: Color::Blue,
            selection_fg: Color::White,
            dim: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
            success: Color::Green,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_default_dark_variant_exposes_all_fields() {
        // Smoke test that Theme::default() returns the dark variant
        // with every documented field populated. Card rule [16].
        let theme = Theme::default();
        assert_eq!(theme.fg, Color::White);
        assert_eq!(theme.bg, Color::Black);
        assert_eq!(theme.border, Color::Reset);
        assert_eq!(theme.border_focused, Color::Cyan);
        assert_eq!(theme.selection_bg, Color::Blue);
        assert_eq!(theme.selection_fg, Color::White);
        assert_eq!(theme.dim, Color::DarkGray);
        assert_eq!(theme.error, Color::Red);
        assert_eq!(theme.warning, Color::Yellow);
        assert_eq!(theme.success, Color::Green);
    }
}
