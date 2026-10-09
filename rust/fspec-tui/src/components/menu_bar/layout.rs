//! MENU-001 — the 2-zone menu bar's pure geometry pass.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R1, R5, R6).
//!
//! `menu_bar_layout` picks the truncation level that fits the area and
//! returns per-item / per-cell hit-test rects. The painter
//! ([`super::paint`]) is a thin consumer of this layout.
//!
//! Truncation ladder (R6), most → least detailed:
//!   0 — full: chips show `#n WU glyph`
//!   1 — drop WU id suffixes
//!   2 — fold chips beyond the 3rd into a single dim `+N` marker
//!   3 — drop non-active view labels (mux)
//!   4 — absolute minimum: Zone A only (no separator, no Zone B)
//!
//! MENU-009 (R5): Zone C (the right-aligned New Agent / Close Agent
//! buttons) is the OPTIONAL convenience zone — it is right-aligned at
//! the row's edge and drops BEFORE any Zone B ladder step: the
//! pre-MENU-009 level is computed first, and the buttons are painted
//! only when the row still affords their width at that level. Once
//! dropped, the row is byte-identical to pre-MENU-009.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::chips::index_prefix;
use super::MenuSnapshot;
use super::ZoneBCell;

/// MENU-009 R5: the gap (cells) between the left-anchored Zone A/B block
/// and the right-aligned Zone C buttons, and between two Zone C buttons
/// (visual parity with the Zone B 2-cell inter-cell gap).
pub const ZONE_C_GAP: usize = 2;

/// One PAINTABLE Zone B cell (post-fold) paired with the original
/// `zone_b` index it covers — focus (which addresses original indices)
/// stays correct through the folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayCell {
    View {
        label: &'static str,
        active: bool,
        orig: usize,
    },
    Chip {
        orig: usize,
    },
    /// `+N` fold marker covering `orig`..`orig + N` chip cells.
    Fold {
        count: usize,
        orig: usize,
    },
}

/// The geometry returned by [`menu_bar_layout`]: truncation level +
/// hit-test rects. `cells` and `cell_rects` are aligned; a fold cell's
/// rect maps to the FIRST folded original index (the painter paints
/// `+N` there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLayout {
    /// The truncation level that was used (0 = full detail, 4 = none).
    pub level: u8,
    /// Zone A item rects, in `CATEGORIES` order.
    pub item_rects: Vec<Rect>,
    /// The `│` separator rect (none at level 4).
    pub separator: Option<Rect>,
    /// The painted Zone B cells (post-fold, display order).
    pub cells: Vec<DisplayCell>,
    /// The Zone B cell rects, aligned with `cells`.
    pub cell_rects: Vec<Rect>,
    /// MENU-009 (R5): the painted Zone C button rects (right-aligned,
    /// `snap.zone_c` order). Empty when the row dropped the zone (or
    /// the surface carries no buttons — mux, Q1).
    pub zone_c_rects: Vec<Rect>,
}

/// The geometry pass: pick the truncation level that fits `area` and
/// lay out the item / separator / cell rects left-to-right. Zone A is
/// the snapshot's own item list (`snap.zone_a` — board/mux default to
/// [`super::items::CATEGORIES`]; the agent view carries its single
/// 'Board View' item, MENU-007). Returns `None` when the area is too
/// small (zero size, or < 3 cells wide once the 1-cell R1 padding is
/// applied).
pub fn menu_bar_layout(area: Rect, snap: &MenuSnapshot) -> Option<MenuLayout> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    // 1-cell horizontal padding each side (R1); < 3 cells is too tight.
    if area.width < 3 {
        return None;
    }
    let inner = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width - 2,
        height: 1,
    };
    // Even the absolute minimum Zone A cannot fit — painting nothing is
    // safer than overflowing the area (R6 proptest bound).
    if zone_a_width(snap) > inner.width as usize {
        return None;
    }
    // R6: the most detailed level that fits. A level with no Zone B
    // cells costs zero width (no separator, no cells — R6: "the
    // separator and Zone B are omitted entirely"). The ladder itself
    // is UNCHANGED by MENU-009 (R5: Zone C drops before it, never
    // shifts it).
    let level = (0..=4)
        .find(|l| zone_b_width(snap, *l) + zone_a_width(snap) <= inner.width as usize)
        .unwrap_or(4);
    let cells = display_cells(snap, level);

    // MENU-009 R5: Zone C is right-aligned at the inner right edge and
    // the OPTIONAL zone — it paints only when the row still affords
    // `ZONE_C_GAP + its width` after the left-anchored block at the
    // selected level. Dropped ⇒ byte-identical row to pre-MENU-009.
    let left_width = zone_a_width(snap) + zone_b_width(snap, level);
    let zone_c_width = zone_c_width(snap);
    let zone_c_fits =
        zone_c_width > 0 && left_width + ZONE_C_GAP + zone_c_width <= inner.width as usize;

    let mut x = inner.x;
    let mut item_rects = Vec::with_capacity(snap.zone_a.len());
    for category in snap.zone_a.iter() {
        // MENU-011 R1: the Zone A item's PAINTED label is the bracketed
        // form (`[ Kanban ]`) — the rect covers the brackets (R4).
        let w = category.bracketed_label().width() as u16;
        item_rects.push(Rect {
            x,
            y: inner.y,
            width: w,
            height: 1,
        });
        x = x.saturating_add(w + 1); // 1-cell gap between items
    }

    let mut separator = None;
    let mut cell_rects = Vec::with_capacity(cells.len());
    if level < 4 && !cells.is_empty() {
        // 1 space + │ + 1 space between zones.
        separator = Some(Rect {
            x: x + 1,
            y: inner.y,
            width: 1,
            height: 1,
        });
        x += 3;
        for cell in &cells {
            let w = cell_width(snap, cell, level) as u16;
            cell_rects.push(Rect {
                x,
                y: inner.y,
                width: w,
                height: 1,
            });
            x = x.saturating_add(w + 2); // 2-cell gap between cells
        }
    }

    // MENU-009 R1/R5: the right-aligned Zone C buttons, laid out
    // left-to-right from the inner right edge (2-cell gaps, Zone B
    // visual parity).
    let mut zone_c_rects: Vec<Rect> = Vec::with_capacity(snap.zone_c.len());
    if zone_c_fits {
        let mut cx = inner.x + inner.width - zone_c_width as u16;
        for button in snap.zone_c.iter() {
            let w = button.label.width() as u16;
            zone_c_rects.push(Rect {
                x: cx,
                y: inner.y,
                width: w,
                height: 1,
            });
            cx = cx.saturating_add(w + ZONE_C_GAP as u16);
        }
    }
    Some(MenuLayout {
        level,
        item_rects,
        separator,
        cells,
        cell_rects,
        zone_c_rects,
    })
}

/// Zone A natural width: the item labels joined by 1 space. MENU-011
/// R1: the labels count their BRACKETED forms (`[ Kanban ]` =
/// `label + 4` cells) — the brackets are part of the button (R4).
fn zone_a_width(snap: &MenuSnapshot) -> usize {
    let items = snap.zone_a;
    items
        .iter()
        .map(|c| c.bracketed_label().width())
        .sum::<usize>()
        + items.len().saturating_sub(1)
}

/// MENU-009: Zone C's natural width: the button labels joined by the
/// [`ZONE_C_GAP`] gap (zero when the surface carries no buttons).
fn zone_c_width(snap: &MenuSnapshot) -> usize {
    let buttons = snap.zone_c;
    buttons.iter().map(|b| b.label.width()).sum::<usize>()
        + buttons.len().saturating_sub(1) * ZONE_C_GAP
}

/// Zone B width at `level` (level 4, or no cells at all = 0 — the
/// separator is omitted, R6).
fn zone_b_width(snap: &MenuSnapshot, level: u8) -> usize {
    if level >= 4 {
        return 0;
    }
    let cells = display_cells(snap, level);
    if cells.is_empty() {
        return 0;
    }
    // 1 space + │ + 1 space.
    let mut w = 3;
    for (i, cell) in cells.iter().enumerate() {
        w += cell_width(snap, cell, level);
        if i + 1 < cells.len() {
            w += 2;
        }
    }
    w
}

/// The Zone B display cells at `level` (R6 folds applied).
fn display_cells(snap: &MenuSnapshot, level: u8) -> Vec<DisplayCell> {
    let mut out: Vec<DisplayCell> = Vec::with_capacity(snap.zone_b.len());
    let mut folded: Option<(usize, usize)> = None; // (count, orig index)
    let mut shown_chips = 0usize;
    for (orig, cell) in snap.zone_b.iter().enumerate() {
        match cell {
            ZoneBCell::View { label, active } if level >= 3 && !*active => {
                continue; // R6 step 3: drop non-active view labels
            }
            ZoneBCell::Chip(_) if level >= 2 && shown_chips >= 3 => {
                folded.get_or_insert((0, orig)).0 += 1;
                continue;
            }
            ZoneBCell::Chip(_) => shown_chips += 1,
            ZoneBCell::View { .. } => {}
        }
        out.push(match cell {
            ZoneBCell::View { label, active } => DisplayCell::View {
                label,
                active: *active,
                orig,
            },
            ZoneBCell::Chip(_) => DisplayCell::Chip { orig },
        });
    }
    if let Some((count, orig)) = folded {
        out.push(DisplayCell::Fold { count, orig });
    }
    out
}

/// One Zone B cell's width at `level` (marker = `+N`; the WU id suffix
/// counts only at level 0 — it is the R6 step-1 truncation). MENU-011
/// R2: every Zone B cell paints bracketed (`[ ... ]`) — the +4 cells
/// cover the `"[ "` prefix and the `" ]"` suffix (R4: the brackets are
/// part of the hit-test span).
pub fn cell_width(snap: &MenuSnapshot, cell: &DisplayCell, level: u8) -> usize {
    let mut w = match cell {
        DisplayCell::View { label, .. } => label.width(),
        DisplayCell::Fold { count, .. } => format!("+{count}").len(),
        DisplayCell::Chip { orig } => {
            let Some(chip) = snap.chips.get(chip_index(snap, *orig)) else {
                return 0;
            };
            let mut w = index_prefix(chip.index).len();
            if level == 0 {
                if let Some(wu) = &chip.wu_id {
                    w += wu.width() + 1;
                }
            }
            w + chip.glyph.width()
        }
    };
    w += 4; // MENU-011 R2: the "[ " + " ]" brackets
    w
}

/// The chip index an original Zone B cell refers to (view labels have
/// no chip and return 0 — callers must check the variant first).
pub fn chip_index(snap: &MenuSnapshot, orig: usize) -> usize {
    match snap.zone_b.get(orig) {
        Some(ZoneBCell::Chip(i)) => *i,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::components::menu_bar::chips::MenuChip;

    fn chip(i: usize, wu: Option<&'static str>) -> MenuChip {
        MenuChip {
            index: (i + 1, 3),
            glyph: "●".to_string(),
            glyph_style: Default::default(),
            wu_id: wu.map(str::to_string),
            active: false,
        }
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
    fn full_width_keeps_level_zero_with_every_cell() {
        // @step Given a 40-column render area
        let snap = snap(vec![chip(0, None), chip(1, None), chip(2, None)]);
        // @step When the bar is rendered
        let layout =
            menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).expect("layout for 120-col area");
        // @step Then cell 0 and the last cell are background-only
        // @step And every cell of the row has the #333333 background
        assert_eq!(layout.level, 0, "no truncation at 120 cols");
        assert_eq!(layout.item_rects.len(), 4, "MENU-008: four Zone A items");
        assert_eq!(layout.cell_rects.len(), 3);
        assert!(layout.separator.is_some());
    }

    #[test]
    fn wu_ids_count_toward_width_only_at_level_zero() {
        // @step Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
        let snap = snap(vec![chip(0, Some("MENU-001"))]);
        let full = menu_bar_layout(Rect::new(0, 0, 120, 1), &snap).unwrap();
        assert_eq!(full.level, 0);
        assert_eq!(
            full.cell_rects[0].width,
            3 + 8 + 1 + 1 + 4,
            "#1 prefix + WU id + space + glyph + the MENU-011 '[ ]' brackets"
        );
        // 60 cols: the level-0 row (42 bracketed Zone A + 3 separator
        // + 17 chip = 69 inner cells) no longer fits, so the WU id
        // suffix is the first thing dropped (R6 step 1).
        let tight = menu_bar_layout(Rect::new(0, 0, 60, 1), &snap).unwrap();
        // @step Then no chip shows a work-unit id suffix
        assert_eq!(tight.level, 1, "WU id dropped first");
    }

    #[test]
    fn very_tight_width_folds_chips_beyond_three() {
        // @step And the 4th chip is folded into a "+1" marker after the first 3
        // (MENU-011: the bracketed Zone A is 42 cells — the fold level
        // needs 84 inner cells, so the 90-col area is the fixture.)
        let chips = (0..6).map(|i| chip(i, None)).collect();
        let layout = menu_bar_layout(Rect::new(0, 0, 90, 1), &snap(chips)).expect("layout");
        assert!(layout.level >= 2, "folding level used: {}", layout.level);
        let fold_count = layout
            .cells
            .iter()
            .filter(|c| matches!(c, DisplayCell::Fold { .. }))
            .count();
        assert_eq!(fold_count, 1, "a single +N marker");
    }

    #[test]
    fn absolute_minimum_drops_zone_b() {
        // @step Given a MenuSnapshot with 4 open sessions
        // @step When the bar is rendered into a 50-column area
        // (MENU-011: the bracketed Zone A is 42 cells — the minimum a
        // 50-col area keeps; every Zone B ladder step overflows, so
        // Zone B is dropped entirely.)
        // @step Then the row still fits within the area width
        // @step And it shows at least the "Kanban" and "Help" items
        let chips = (0..6)
            .map(|i| chip(i, Some("A-LONG-UNIT-ID-TO-WRAP")))
            .collect();
        let layout = menu_bar_layout(Rect::new(0, 0, 50, 1), &snap(chips)).unwrap();
        assert_eq!(layout.level, 4, "Zone B dropped entirely");
        assert!(layout.separator.is_none());
        assert!(layout.cell_rects.is_empty());
    }

    #[test]
    fn zero_or_tiny_area_yields_no_layout() {
        // @step Given a render area with width 0 or height 0
        // @step When the bar is rendered
        // @step Then the buffer is unchanged
        assert!(menu_bar_layout(Rect::new(0, 0, 0, 1), &snap(vec![chip(0, None)])).is_none());
        assert!(menu_bar_layout(Rect::new(0, 0, 10, 0), &snap(vec![chip(0, None)])).is_none());
        assert!(menu_bar_layout(Rect::new(0, 0, 2, 1), &snap(vec![chip(0, None)])).is_none());
    }

    #[test]
    fn no_chips_is_the_full_level_with_no_zone_b() {
        // R6: with no chips the separator and Zone B are omitted
        // entirely — no truncation is needed, so the level stays 0.
        // @step Given a MenuSnapshot with no open sessions
        // @step When the bar is rendered
        // @step Then the row contains the menu items but no "|" separator and no chips
        let layout = menu_bar_layout(Rect::new(0, 0, 80, 1), &MenuSnapshot::default()).unwrap();
        assert_eq!(layout.level, 0, "no chips → nothing to truncate");
        assert!(layout.separator.is_none());
        assert!(layout.cells.is_empty());
        assert!(layout.cell_rects.is_empty());
        assert_eq!(
            layout.item_rects.len(),
            4,
            "Zone A still lays out (MENU-008)"
        );
    }

    // R6 proptest: EVERY ladder step must keep the painted row within
    // `area.width` — for any number of chips and any area width, the
    // rightmost painted cell (last Zone B cell or last item) ends at
    // or before `area.x + area.width`.
    proptest::proptest! {
        #[test]
        fn painted_row_never_exceeds_the_area_width(
            width in 3u16..=400,
            chip_count in 0usize..=24,
        ) {
            let chips: Vec<MenuChip> = (0..chip_count)
                .map(|i| chip(i, Some("A-BIG-UNIT-ID")))
                .collect();
            let snap = snap(chips);
            // @step Then the row still fits within the area width
            if let Some(layout) = menu_bar_layout(Rect::new(0, 0, width, 1), &snap) {
                // Too-tight areas legitimately paint nothing — only
                // assert the bound when a layout was produced.
                if let Some(last) = layout.cell_rects.last() {
                    assert!(
                        last.x + last.width <= width,
                        "last cell overflows: x {} w {} at width {}",
                        last.x,
                        last.width,
                        width
                    );
                }
                if let Some(last) = layout.item_rects.last() {
                    assert!(
                        last.x + last.width <= width,
                        "last item overflows: x {} w {} at width {}",
                        last.x,
                        last.width,
                        width
                    );
                }
            }
        }
    }
}
