//! Header strip orchestrator — composes the RPC-015 header widgets
//! (`logo` + `checkpoint_status`) into the 4-row strip. Row 3 (the
//! former keybinding-chord row) is EXPOSED, not painted: MENU-002
//! replaces the 'u Actions' chord with the 2-zone menu bar, which the
//! board render path paints into this row.
//!
//! Feature: spec/features/rpc015-board-header.feature (rows 0-2)
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature (row 3)
//!
//! Mirrors the TS layout from `src/tui/components/UnifiedBoardLayout.tsx:360-380`:
//!   left column  — 12 cells wide — multi-line FSPEC logo
//!   right column — fills the rest — checkpoint status + divider + bar row

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::store::BoardStore;
use crate::theme::Theme;

use super::checkpoint_status;
use super::logo;

/// The two painted rows + the EXPOSED row 3 the menu bar paints into.
pub struct HeaderRows {
    /// The former keybinding-chord row (row 3 of the strip).
    pub bar_row: Rect,
}

/// Paint rows 0-2 of the header strip (`logo` + `checkpoint_status` +
/// the `─` divider) and return row 3's rect for the 2-zone menu bar
/// (MENU-002 — the 'u Actions' chord no longer paints here). `area` is
/// the inner rectangle BETWEEN the left and right `│` border columns;
/// `area.height` must be at least 4.
pub fn paint(area: Rect, buf: &mut Buffer, store: &BoardStore, theme: &Theme) -> HeaderRows {
    // Fallback for too-small strips (never painted, never hit-tested).
    let empty = Rect::new(area.x, area.y.saturating_add(3), 0, 1);
    if area.width == 0 || area.height < 4 {
        return HeaderRows {
            bar_row: Rect::new(area.x, area.y, 0, 1),
        };
    }
    if area.width < 3 {
        return HeaderRows { bar_row: empty };
    }
    // Apply the `paddingX={1}` from the TS layout: 1 cell of breathing
    // room between the left `│` border and the logo, and 1 cell between
    // the bar row and the right `│` border.
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
    // BOARD-021: pass the theme so the logo's 4th row (build version)
    // paints in the dim color.
    logo::render(left, buf, theme);
    // The right column begins immediately after the logo block. The TS
    // logo glyph row already ends with a trailing space, so no extra
    // padding cell is needed here.
    let right_start_x = padded.x.saturating_add(logo_w);
    let padded_end = padded.x + padded.width;
    if right_start_x >= padded_end {
        return HeaderRows { bar_row: empty };
    }
    let right_w = padded_end - right_start_x;
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
    // Row 3: EXPOSED — the 2-zone menu bar paints into it (MENU-002).
    HeaderRows {
        bar_row: Rect {
            x: right_start_x,
            y: padded.y + 3,
            width: right_w,
            height: 1,
        },
    }
}
