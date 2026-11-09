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
//! MENU-009 (R5): Zone C (the right-aligned New Agent / Close
//! buttons) is the OPTIONAL convenience zone — it is right-aligned at
//! the row's edge and drops BEFORE any Zone B ladder step: the
//! pre-MENU-009 level is computed first, and the buttons are painted
//! only when the row still affords their width at that level. Once
//! dropped, the row is byte-identical to pre-MENU-009.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use super::chips::chip_prefix;
use super::MenuSnapshot;
use super::ZoneBCell;
use super::ZoneCButton;

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
    // TUI-114: when every button does not fit, the leading (leftmost)
    // buttons drop first — `New Bot` before `New Agent` / `Close` —
    // keeping the rest right-aligned. Dropped buttons keep a zero-width
    // rect so `zone_c_rects` stays index-aligned with `snap.zone_c`.
    let left_width = zone_a_width(snap) + zone_b_width(snap, level);
    let dropped = (0..snap.zone_c.len())
        .find(|skip| {
            let width = buttons_width(&snap.zone_c[*skip..]);
            width > 0 && left_width + ZONE_C_GAP + width <= inner.width as usize
        })
        .unwrap_or(snap.zone_c.len());
    let zone_c_width = buttons_width(&snap.zone_c[dropped..]);
    let zone_c_fits = zone_c_width > 0;

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
        for (index, button) in snap.zone_c.iter().enumerate() {
            if index < dropped {
                zone_c_rects.push(Rect {
                    x: cx,
                    y: inner.y,
                    width: 0,
                    height: 1,
                });
                continue;
            }
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

include!("layout_widths.rs");

#[cfg(test)]
mod tests;
