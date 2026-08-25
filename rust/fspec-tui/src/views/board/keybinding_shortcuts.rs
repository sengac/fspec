//! Top-level keybinding chord shortcuts widget — Rust port of
//! `src/tui/components/KeybindingShortcuts.tsx`.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015.
//!
//! Paints the literal chord line:
//!   `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search`
//!
//! The C / F / D keybindings are hint-only in this card; the `.` New
//! Agent binding is wired in RPC-395 (opens AgentView) and the `/`
//! Search binding in BOARD-022 (work-unit search dialog).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::Theme;

/// Render the 1-row keybinding chord into `area`. `area.height` must
/// be at least 1.
pub fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // TS source (`KeybindingShortcuts.tsx`) renders a single plain
    // `<Text>` with no color/bold attributes — so port it as one span
    // styled with the theme's primary fg.
    let style = Style::default().fg(theme.fg);
    let line = Line::from(Span::styled(
        "C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search".to_string(),
        style,
    ));
    Paragraph::new(line).render(
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
        buf,
    );
}
