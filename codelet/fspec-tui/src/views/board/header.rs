//! Header strip orchestrator — composes the three RPC-015 header widgets
//! (`logo` + `checkpoint_status` + `keybinding_shortcuts`) into a single
//! 4-row strip.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015.
//!
//! Mirrors the TS layout from `src/tui/components/UnifiedBoardLayout.tsx:360-380`:
//!   left column  — 12 cells wide — multi-line FSPEC logo
//!   right column — fills the rest — checkpoint status + divider + chord

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::store::BoardStore;
use crate::theme::Theme;

use super::checkpoint_status;
use super::keybinding_shortcuts;
use super::logo;

/// Paint the 4-row header strip into `area`. `area.height` must be at
/// least 4; smaller heights produce nothing.
pub fn paint(area: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    if area.width == 0 || area.height < 4 {
        return;
    }
    let logo_w = logo::LOGO_WIDTH.min(area.width);
    let left = Rect { x: area.x, y: area.y, width: logo_w, height: area.height };
    logo::render(left, buf);
    // 1 cell of padding between left + right columns matches the
    // `<Box paddingX={1}>` on the TS side.
    let right_start_x = area.x.saturating_add(logo_w).saturating_add(1);
    if right_start_x >= area.x + area.width {
        return;
    }
    let right_w = area.x + area.width - right_start_x;
    if right_w == 0 {
        return;
    }
    // Row 1: checkpoint status.
    let row1 = Rect { x: right_start_x, y: area.y + 1, width: right_w, height: 1 };
    checkpoint_status::render(row1, buf, store.checkpoint_counts());
    // Row 2: `─` divider line (TS `borderTop` on KeybindingShortcuts).
    let divider_y = area.y + 2;
    let divider_style = Style::default().fg(theme.border);
    for x in right_start_x..(right_start_x + right_w) {
        buf.set_string(x, divider_y, "─", divider_style);
    }
    // Row 3: keybinding chord.
    let row3 = Rect { x: right_start_x, y: area.y + 3, width: right_w, height: 1 };
    keybinding_shortcuts::render(row3, buf, theme);
}
