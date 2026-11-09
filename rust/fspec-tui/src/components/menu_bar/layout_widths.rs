// Width helpers for the menu-bar layout, `include!`d into layout.rs so
// their paths are unchanged.

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
fn buttons_width(buttons: &[ZoneCButton]) -> usize {
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
            let mut w = chip_prefix(chip).len();
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
