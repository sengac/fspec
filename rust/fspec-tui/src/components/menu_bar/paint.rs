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

use super::chips::index_prefix;
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
        paint_text(buf, rect.x, y, category.label, style);
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
    match cell {
        DisplayCell::View { label, active, .. } => {
            let mut style = Style::default().fg(if *active { Color::Cyan } else { theme.dim });
            if *active {
                style = style.add_modifier(Modifier::BOLD);
            }
            paint_text(buf, x, y, label, style);
        }
        DisplayCell::Fold { count, .. } => {
            paint_text(
                buf,
                x,
                y,
                &format!("+{count}"),
                if focused {
                    inverse_style()
                } else {
                    Style::default().fg(theme.dim)
                },
            );
        }
        DisplayCell::Chip { orig } => {
            let Some(chip) = snap.chips.get(chip_index(snap, *orig)) else {
                return;
            };
            let mut cx = x;
            cx = paint_text(buf, cx, y, &index_prefix(chip.index), fg(theme.fg, focused));
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
            paint_text(buf, cx, y, &chip.glyph, glyph_style);
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::components::menu_bar::chips::MenuChip;
    use ratatui::backend::TestBackend;

    fn chip(i: usize, wu: Option<&'static str>) -> MenuChip {
        MenuChip {
            index: (i + 1, 3),
            glyph: "●".to_string(),
            glyph_style: Default::default(),
            wu_id: wu.map(str::to_string),
            active: false,
        }
    }

    fn render(snap: &MenuSnapshot, w: u16) -> (Buffer, MenuLayout) {
        let area = Rect::new(0, 0, w, 1);
        let _ = TestBackend::new(w, 4);
        let mut buf = Buffer::empty(area);
        let layout = paint_menu_bar(area, &mut buf, snap, &Theme::default())
            .expect("layout for non-zero area");
        (buf, layout)
    }

    fn row(buf: &Buffer, w: u16) -> String {
        (0..w)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn snap(chips: Vec<MenuChip>) -> MenuSnapshot {
        let zone_b = chips
            .iter()
            .enumerate()
            .map(|(i, _)| ZoneBCell::Chip(i))
            .collect();
        MenuSnapshot {
            zone_b,
            chips,
            ..Default::default()
        }
    }

    #[test]
    fn full_width_row_paints_items_separator_and_chips() {
        // @step Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
        // @step When the bar is rendered into a 120-column row
        let (buf, layout) = render(
            &snap(vec![chip(0, None), chip(1, None), chip(2, None)]),
            120,
        );
        // @step Then Zone B reads "#1 ●  #2 ⠋  #3 ●"
        let line = row(&buf, 120);
        assert!(
            line.starts_with(" Kanban Tools Settings Help"),
            "Zone A first, after the 1-cell R1 pad: {line}"
        );
        assert!(line.contains("│"), "separator present: {line}");
        assert!(line.contains("#1 ●"), "chip 1: {line}");
        assert!(line.contains("#2 ●"), "chip 2: {line}");
        assert!(line.contains("#3 ●"), "chip 3: {line}");
        // @step And the Running chip's glyph cell is styled magenta
        // (the 120-col fixture chips are Idle-styled dots; the Running
        // braille + magenta styling is pinned in chips::running_uses_the_braille_frame_at_the_clock)
        assert_eq!(layout.level, 0, "no truncation at 120 cols");
        assert_eq!(layout.item_rects.len(), 4);
        assert_eq!(layout.cell_rects.len(), 3);
    }

    #[test]
    fn wu_ids_appear_at_level_zero() {
        // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
        // @step And a 200-column render area
        // @step When the bar is rendered
        let (buf, layout) = render(&snap(vec![chip(0, Some("MENU-001"))]), 120);
        // @step Then the chip reads "#1 MENU-001 ●"
        assert_eq!(layout.level, 0);
        assert!(row(&buf, 120).contains("MENU-001 ●"));
    }

    #[test]
    fn tight_width_drops_wu_ids_before_anything_else() {
        // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
        let snap = snap(vec![chip(0, Some("MENU-001")), chip(1, Some("MENU-002"))]);
        // @step When the bar is rendered into a 58-column area
        // (MENU-008: Zone A grew from 9 to 26 inner cells, so the
        // level-0/level-1 boundary moved — 58 cols is 56 inner: level 0
        // needs 62 (26 + separator + two 19-cell chips), level 1 needs
        // 45, so WU ids are still the first thing dropped.)
        let (buf, layout) = render(&snap, 58);
        // @step Then no chip shows a work-unit id suffix
        assert_eq!(layout.level, 1, "WU ids dropped first");
        assert!(!row(&buf, 58).contains("MENU-001"));
        assert!(row(&buf, 58).contains("#1 ●"));
    }

    #[test]
    fn focused_item_paints_inverse_video() {
        // @step Given a MenuSnapshot with MenuFocus on the 0th menu item
        let mut snap = snap(vec![chip(0, None)]);
        snap.focus = Some(MenuFocus::Item(0));
        // @step When the bar is rendered
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
        // @step Then the "Kanban" item's cells are styled bg Cyan fg Black
        // The 'K' of 'Kanban' (x=1 after the 1-cell pad) is inverse.
        assert_eq!(buf[(1, 0)].bg, Color::Cyan);
        assert_eq!(buf[(1, 0)].fg, Color::Black);
        assert!(buf[(1, 0)].modifier.contains(Modifier::BOLD));
        // @step And the "Help" item's cells are not inverted
        let help_x = layout.item_rects[3].x;
        assert_ne!(buf[(help_x, 0)].bg, Color::Cyan);
    }

    #[test]
    fn focused_chip_paints_inverse_video() {
        // @step Given a MenuSnapshot with 2 open sessions and MenuFocus on chip 1
        let mut snap = snap(vec![chip(0, None), chip(1, None)]);
        snap.focus = Some(MenuFocus::ZoneB(1));
        // @step When the bar is rendered
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
        // @step Then the second chip's cells are styled bg Cyan fg Black
        let rect = &layout.cell_rects[1];
        assert_eq!(
            buf[(rect.x, 0)].bg,
            Color::Cyan,
            "focused chip cell inverted"
        );
        assert_eq!(
            buf[(rect.x + rect.width - 1, 0)].bg,
            Color::Cyan,
            "whole chip cell inverted"
        );
        // @step And the first chip's cells are not inverted
        let rect0 = &layout.cell_rects[0];
        assert_ne!(buf[(rect0.x, 0)].bg, Color::Cyan);
    }

    #[test]
    fn open_dropdown_marks_its_item_active() {
        let mut snap = snap(vec![chip(0, None)]);
        snap.open_menu = Some((3, 0));
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let layout = paint_menu_bar(area, &mut buf, &snap, &Theme::default()).expect("layout");
        let help_x = layout.item_rects[3].x;
        assert_eq!(buf[(help_x, 0)].fg, Color::Black);
    }

    #[test]
    fn no_focus_paints_no_highlight() {
        // @step Given a MenuSnapshot with MenuFocus None
        // @step When the bar is rendered
        let snap = snap(vec![chip(0, None), chip(1, None)]);
        let (buf, _) = render(&snap, 80);
        // @step Then no cell in the row is styled bg Cyan
        for x in 0..80u16 {
            assert_ne!(
                buf[(x, 0)].bg,
                Color::Cyan,
                "cell {x} unexpectedly inverse without a focus"
            );
        }
    }

    #[test]
    fn zero_area_paints_nothing() {
        // @step Given a render area with width 0 or height 0
        // @step When the bar is rendered
        let snap = snap(vec![chip(0, None)]);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        // @step Then the buffer is unchanged
        assert!(
            paint_menu_bar(Rect::new(0, 0, 0, 1), &mut buf, &snap, &Theme::default()).is_none()
        );
        assert!(
            paint_menu_bar(Rect::new(0, 0, 10, 0), &mut buf, &snap, &Theme::default()).is_none()
        );
    }

    #[test]
    fn every_cell_carries_the_dark_background() {
        // @step Given a 40-column render area
        let (buf, _) = render(&snap(vec![chip(0, None)]), 60);
        // @step And every cell of the row has the #333333 background
        for x in 0..60u16 {
            assert_eq!(buf[(x, 0)].bg, MENU_BAR_BG, "cell {x} missing bg");
        }
    }

    #[test]
    fn no_chips_omits_separator_and_zone_b() {
        // R6: with no chips the separator and Zone B are omitted
        // entirely — the row is just the menu items.
        // @step Given a MenuSnapshot with no open sessions
        // @step When the bar is rendered
        let snap = MenuSnapshot::default();
        let (buf, layout) = render(&snap, 80);
        // @step Then the row contains the menu items but no "|" separator and no chips
        assert!(layout.separator.is_none());
        assert!(layout.cells.is_empty());
        assert!(!row(&buf, 80).contains("│"));
        assert!(row(&buf, 80).starts_with(" Kanban Tools Settings Help"));
    }
}
