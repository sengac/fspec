//! RPC-054 — the status line's style.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! BUG-578 F22: the Detail::Summary status enum went with the Summary view.

use ratatui::style::{Color, Style};

/// BUG-575: a status line's style by its outcome: a `✓` message is green
/// and a `✗` message red; any other text (a form hint) keeps `unmarked`.
pub(super) fn status_style(status: &str, unmarked: Style) -> Style {
    if status.starts_with('✓') {
        Style::default().fg(Color::Green)
    } else if status.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        unmarked
    }
}
