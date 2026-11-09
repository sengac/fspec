//! MENU-001 — the 1-row menu bar painter.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R1, R5, R6).
//!
//! `paint_menu_bar` paints the row (R1: #333333 bg on every cell,
//! 1-cell horizontal padding, dim `│` separator) and returns the
//! [`layout::MenuLayout`] for hit-testing. The pure geometry pass +
//! the R6 truncation ladder live in [`super::layout`].

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};

use crate::theme::Theme;

use super::chips::chip_prefix;
use super::layout::{cell_width, chip_index, menu_bar_layout, DisplayCell, MenuLayout};
use super::{MenuFocus, MenuSnapshot, ZoneBCell};

/// The bar's row background (R1 — SessionHeader/SessionFooter parity).
pub const MENU_BAR_BG: Color = Color::Rgb(0x33, 0x33, 0x33);

/// R5: the focused/open item highlight (inverse-video).
fn inverse_style() -> Style {
    Style::default()
        .bg(Color::Cyan)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

/// Paint the menu bar into `area` and return the layout (R1). Returns
/// `None` (painting nothing) when the area is too small.
pub fn paint_menu_bar(
    area: Rect,
    buf: &mut Buffer,
    snap: &MenuSnapshot,
    theme: &Theme,
) -> Option<MenuLayout> {
    let layout = menu_bar_layout(area, snap)?;
    // R1: the #333333 background on EVERY cell of the row.
    for x in area.x..area.x + area.width {
        if buf.area.contains(Position::new(x, area.y)) {
            buf[(x, area.y)].set_symbol(" ");
            buf[(x, area.y)].set_bg(MENU_BAR_BG);
        }
    }
    let y = area.y;
    // Zone A items (the snapshot's own list — MENU-007: the agent
    // view carries a single 'Board View' item, board/mux carry the
    // full registry).
    for (i, rect) in layout.item_rects.iter().enumerate() {
        let Some(category) = snap.zone_a.get(i) else {
            continue;
        };
        let active = snap
            .focus
            .is_some_and(|f| matches!(f, MenuFocus::Item(f) if f == i))
            || snap.open_menu.is_some_and(|(c, _)| c == i);
        let style = if active {
            inverse_style()
        } else {
            Style::default().fg(theme.fg)
        };
        // MENU-011 R1: Zone A items paint their BRACKETED label
        // (`[ Kanban ]`) — the brackets are part of the button, so the
        // focused inverse-video highlight covers them too (R4).
        paint_text(buf, rect.x, y, &category.bracketed_label(), style);
    }
    // Zone B (separator + cells) when the level keeps it. (No separator
    // = no cells to paint — the pre-MENU-009 early return, now folded
    // into the `if let` so the Zone C arm below ALWAYS runs.)
    if layout.level < 4 {
        if let Some(sep) = layout.separator {
            paint_text(buf, sep.x, y, "│", Style::default().fg(theme.dim));
            for (i, rect) in layout.cell_rects.iter().enumerate() {
                let Some(cell) = layout.cells.get(i) else {
                    continue;
                };
                let focused = focused_cell(snap, cell);
                paint_cell(buf, snap, cell, rect.x, y, layout.level, theme, focused);
            }
        }
    }
    // MENU-009 R1/R4: the right-aligned Zone C buttons. Plain `theme.fg`
    // when unfocused (they read as actionable buttons, not chips), the
    // SAME inverse-video highlight as Zone A items when the ring focus
    // lands on them (R4). The #333333 row bg (R1) already covers the
    // area — no extra fill needed.
    for (i, rect) in layout.zone_c_rects.iter().enumerate() {
        let Some(button) = snap.zone_c.get(i) else {
            continue;
        };
        // TUI-114: a button dropped for width paints nothing.
        if rect.width == 0 {
            continue;
        }
        let focused = matches!(snap.focus, Some(MenuFocus::ZoneC(f)) if f == i);
        if focused {
            // R4: the whole button goes inverse first, then the text
            // (the focused_cell inverse-first pattern).
            for cx in rect.x..rect.x.saturating_add(rect.width) {
                if buf.area.contains(Position::new(cx, y)) {
                    buf[(cx, y)].set_symbol(" ");
                    buf[(cx, y)].set_style(inverse_style());
                }
            }
        }
        paint_text(
            buf,
            rect.x,
            y,
            button.label,
            if focused {
                inverse_style()
            } else {
                Style::default().fg(theme.fg)
            },
        );
    }
    Some(layout)
}

/// R5 + MENU-006: true when the cell should paint the selected-item
/// highlight — either the ring focus lands on it (chips / views by
/// original index, fold markers when ANY folded cell is focused) OR the
/// chip is the store's CURRENT session (the active-chip highlight,
/// independent of the ring).
fn focused_cell(snap: &MenuSnapshot, cell: &DisplayCell) -> bool {
    if let DisplayCell::Chip { orig } = cell {
        // MENU-006: the active (current) session's chip highlights even
        // with no ring focus — the "blue background" the user asked for.
        if let Some(chip) = snap.chips.get(chip_index(snap, *orig)) {
            if chip.active {
                return true;
            }
        }
    }
    let Some(MenuFocus::ZoneB(f)) = snap.focus else {
        return false;
    };
    match cell {
        DisplayCell::View { orig, .. } | DisplayCell::Chip { orig } => f == *orig,
        DisplayCell::Fold { count, orig } => (*orig..*orig + *count).any(|o| {
            snap.zone_b
                .get(o)
                .is_some_and(|c| matches!(c, ZoneBCell::Chip(_)))
                && f == o
        }),
    }
}

/// Paints one Zone B cell. `snap`/`cell`/`level` together identify the
/// chip + truncation shape; the rest are the paint target.
#[allow(clippy::too_many_arguments)]
fn paint_cell(
    buf: &mut Buffer,
    snap: &MenuSnapshot,
    cell: &DisplayCell,
    x: u16,
    y: u16,
    level: u8,
    theme: &Theme,
    focused: bool,
) {
    let width = cell_width(snap, cell, level);
    if focused {
        // R5: the whole cell goes inverse first, then the text.
        for cx in x..x.saturating_add(width as u16) {
            if buf.area.contains(Position::new(cx, y)) {
                buf[(cx, y)].set_symbol(" ");
                buf[(cx, y)].set_style(inverse_style());
            }
        }
    }
    // MENU-011 R2: every Zone B cell paints BRACKETED (`[ ... ]`) — the
    // brackets carry the cell's own style; the inner content keeps its
    // per-segment styling. The layout's `+4` width accounts for them
    // (R4: the brackets are part of the hit-test span + the inverse
    // highlight when focused).
    match cell {
        DisplayCell::View { label, active, .. } => {
            // BUG-202 R1: the ACTIVE (focused-pane) label paints BOLD +
            // theme.fg (white) — never a standalone cyan foreground (it
            // blended into the blue-tinted bar background). The
            // inverse-video highlight (bg Cyan / fg Black) stays the
            // ONLY cyan in the bar row, so 'focused pane' and
            // 'ring-focused' remain visually distinguishable.
            let mut style = Style::default().fg(if *active { theme.fg } else { theme.dim });
            if *active {
                style = style.add_modifier(Modifier::BOLD);
            }
            let mut cx = paint_text(buf, x, y, "[ ", style);
            cx = paint_text(buf, cx, y, label, style);
            paint_text(buf, cx, y, " ]", style);
        }
        DisplayCell::Fold { count, .. } => {
            let style = if focused {
                inverse_style()
            } else {
                Style::default().fg(theme.dim)
            };
            let mut cx = paint_text(buf, x, y, "[ ", style);
            cx = paint_text(buf, cx, y, &format!("+{count}"), style);
            paint_text(buf, cx, y, " ]", style);
        }
        DisplayCell::Chip { orig } => {
            let Some(chip) = snap.chips.get(chip_index(snap, *orig)) else {
                return;
            };
            let mut cx = paint_text(buf, x, y, "[ ", fg(theme.fg, focused));
            cx = paint_text(buf, cx, y, &chip_prefix(chip), fg(theme.fg, focused));
            // R6 step 1: the WU suffix paints only at level 0.
            if level == 0 {
                if let Some(wu) = &chip.wu_id {
                    cx = paint_text(buf, cx, y, wu, fg(theme.dim, focused));
                    cx = paint_text(buf, cx, y, " ", Style::default());
                }
            }
            let glyph_style = if focused {
                inverse_style()
            } else {
                chip.glyph_style
            };
            cx = paint_text(buf, cx, y, &chip.glyph, glyph_style);
            paint_text(buf, cx, y, " ]", fg(theme.fg, focused));
        }
    }
}

/// Cell style with the focused-inverse fg override when focused.
fn fg(color: Color, focused: bool) -> Style {
    if focused {
        inverse_style()
    } else {
        Style::default().fg(color)
    }
}

/// Paint `text` at (x, y) — single style per call (the caller splits
/// multi-style cells into calls). Returns the x past the last cell.
fn paint_text(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) -> u16 {
    let mut cx = x;
    for ch in text.chars() {
        if !buf.area.contains(Position::new(cx, y)) {
            break;
        }
        buf[(cx, y)].set_char(ch);
        buf[(cx, y)].set_style(style);
        cx = cx.saturating_add(1);
    }
    cx
}

#[cfg(test)]
mod tests;
