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
///
/// `area` is the inner rectangle BETWEEN the left and right `│` border
/// columns. The TS source wraps the logo + right column in a `<Box
/// paddingX={1}>` — so we shave one cell off the left and right edges
/// of `area` before laying out the children.
pub fn paint(area: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) {
    if area.width == 0 || area.height < 4 {
        return;
    }
    // Apply the `paddingX={1}` from the TS layout: 1 cell of breathing
    // room between the left `│` border and the logo, and 1 cell between
    // the keybinding chord and the right `│` border.
    if area.width < 3 {
        return;
    }
    let padded = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width - 2,
        height: area.height,
    };
    let logo_w = logo::LOGO_WIDTH.min(padded.width);
    let left = Rect {
        x: padded.x,
        y: padded.y,
        width: logo_w,
        height: padded.height,
    };
    logo::render(left, buf);
    // The right column begins immediately after the logo block. The TS
    // logo glyph row already ends with a trailing space, so no extra
    // padding cell is needed here.
    let right_start_x = padded.x.saturating_add(logo_w);
    let padded_end = padded.x + padded.width;
    if right_start_x >= padded_end {
        return;
    }
    let right_w = padded_end - right_start_x;
    if right_w == 0 {
        return;
    }
    // Row 0: checkpoint status (matches TS: <CheckpointStatus /> is the
    // first child of the right-hand column → top row of the header).
    let row0 = Rect {
        x: right_start_x,
        y: padded.y,
        width: right_w,
        height: 1,
    };
    checkpoint_status::render(row0, buf, store.checkpoint_counts());
    // Row 2: `─` divider line (TS `borderTop` on KeybindingShortcuts).
    let divider_y = padded.y + 2;
    let divider_style = Style::default().fg(theme.border);
    for x in right_start_x..(right_start_x + right_w) {
        buf.set_string(x, divider_y, "─", divider_style);
    }
    // Row 3: keybinding chord.
    let row3 = Rect {
        x: right_start_x,
        y: padded.y + 3,
        width: right_w,
        height: 1,
    };
    keybinding_shortcuts::render(row3, buf, theme);
}
