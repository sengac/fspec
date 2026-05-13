//! Top-level keybinding chord shortcuts widget — Rust port of
//! `src/tui/components/KeybindingShortcuts.tsx`.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015.
//!
//! Paints the literal chord line:
//!   `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`
//!
//! The C / F / D / / keybindings are hint-only in this card — wiring
//! lands in subsequent RPC-002 children.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::Theme;

/// Render the 1-row keybinding chord into `area`. `area.height` must
/// be at least 1.
pub fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let key = Style::default().fg(theme.fg).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.dim);
    let line = Line::from(vec![
        Span::styled("C ", key),
        Span::styled("Checkpoints ", dim),
        Span::styled("◆ ", dim),
        Span::styled("F ", key),
        Span::styled("Changed Files ", dim),
        Span::styled("◆ ", dim),
        Span::styled("D ", key),
        Span::styled("FOUNDATION.md ", dim),
        Span::styled("◆ ", dim),
        Span::styled("/ ", key),
        Span::styled("New Agent", dim),
    ]);
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
