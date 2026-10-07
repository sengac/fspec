//! TUI-112 — shared hit-test geometry for modal-dialog click activation.
//!
//! Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
//! Card: TUI-112.
//!
//! Every Enter-selectable modal dialog must ALSO commit a choice on a
//! mouse left-click. This module provides the shared geometry so each
//! dialog hit-tests against EXACTLY what `dialog_theme::render_dialog_at`
//! paints (R4/R5): a click on a control commits it; a click on a
//! non-activatable cell (gap, title, footer, border, outside the dialog)
//! is ignored.
//!
//! The content-row math below MIRRORS `render_dialog_at`'s spacious /
//! compact / footer / query-row layout. If that function changes, update
//! this helper in lockstep (they must agree on the painted Y of each
//! content row or click activation silently drifts from the pixels).

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use crate::components::dialog_theme::FspecDialog;
use crate::mouse::rect_contains;
/// The last-rendered control geometry a dialog uses for click hit-testing.
///
/// A dialog captures this once per `render()` (so it reflects the exact
/// pixels painted) and, on a `Down(Left)`, hit-tests the click against
/// `controls`: a hit commits that control; a miss is `Ignored` (R5).
/// `dialog_rect` is the full dialog frame (R4): clicks outside it are
/// ignored so they bubble to whatever is behind the modal.
#[derive(Debug, Clone, Default)]
pub struct LastLayout {
    /// The dialog frame from the last render (or `None` before render).
    pub dialog_rect: Option<Rect>,
    /// The clickable control rects (buttons or rows), in paint order.
    pub controls: Vec<Rect>,
}

impl LastLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// True iff `(col, row)` lies inside the dialog frame. `false` before
    /// the first render (R4: nothing to hit-test yet).
    pub fn contains(&self, col: u16, row: u16) -> bool {
        match self.dialog_rect {
            Some(r) => rect_contains(r, col, row),
            None => false,
        }
    }

    /// The index of the first control under `(col, row)`, or `None`.
    pub fn hit(&self, col: u16, row: u16) -> Option<usize> {
        hit_button(&self.controls, col, row)
    }
}

/// The inner body rect of a dialog painted at `rect` (inside the border +
/// 1-cell padding), or `None` when `rect` is too small. Mirrors
/// `render_dialog_at`: `Block::inner` then a 1-cell inset.
pub fn dialog_body(rect: Rect) -> Option<Rect> {
    if rect.width < 4 || rect.height < 4 {
        return None;
    }
    Some(Rect {
        x: rect.x.saturating_add(2),
        y: rect.y.saturating_add(2),
        width: rect.width.saturating_sub(4),
        height: rect.height.saturating_sub(4),
    })
}

/// The number of footer lines `footer` occupies (0 when empty). Matches
/// `dialog_theme::footer_line_count`.
pub fn footer_line_count(footer: &str) -> u16 {
    if footer.is_empty() {
        0
    } else {
        footer.lines().count() as u16
    }
}

/// The Y coordinate of the i-th content row for a dialog painted at
/// `rect`, or `None` when that row is clipped off (not painted).
///
/// Mirrors `render_dialog_at`: the body origin, the spacious/compact
/// fallback, the footer reservation, and the optional pinned query row.
/// `row_index` counts content rows AFTER any pinned query row.
pub fn content_row_y(
    rect: Rect,
    footer_h: u16,
    has_query_row: bool,
    row_index: usize,
) -> Option<u16> {
    let body = dialog_body(rect)?;
    if body.width == 0 || body.height == 0 {
        return None;
    }
    let query_h = if has_query_row { 1 } else { 0 };
    let spacious_min = 3 + query_h + if footer_h > 0 { footer_h + 1 } else { 0 };
    let spacious = body.height >= spacious_min;
    let footer_h = if footer_h == 0 {
        0
    } else if spacious || body.height >= 2 + footer_h {
        footer_h
    } else {
        0
    };
    let content_start = body.y + if spacious { 2 } else { 1 };
    let reserved = if footer_h > 0 {
        if spacious {
            footer_h + 1
        } else {
            footer_h
        }
    } else {
        0
    };
    let body_row_end = if footer_h > 0 {
        body.y + body.height.saturating_sub(reserved)
    } else {
        body.y + body.height
    };
    let first_content_y = content_start + query_h;
    let y = first_content_y.saturating_add(row_index as u16);
    if y < body_row_end {
        Some(y)
    } else {
        None
    }
}

/// The full-width rect of the i-th content row for a dialog painted at
/// `rect`, or `None` when that row is clipped off. Full-width means a
/// click anywhere on the row (within the body) targets it — used by
/// row-list dialogs.
pub fn content_row_rect(
    rect: Rect,
    footer_h: u16,
    has_query_row: bool,
    row_index: usize,
) -> Option<Rect> {
    let body = dialog_body(rect)?;
    let y = content_row_y(rect, footer_h, has_query_row, row_index)?;
    Some(Rect {
        x: body.x,
        y,
        width: body.width,
        height: 1,
    })
}

/// The index of the first control rect in `rects` that contains `(col,
/// row)`, or `None` when the click misses every control (R5: such clicks
/// are ignored). Uses half-open containment so a click on the gap between
/// two buttons never bleeds onto either.
pub fn hit_button(rects: &[Rect], col: u16, row: u16) -> Option<usize> {
    rects.iter().position(|r| rect_contains(*r, col, row))
}

/// The leading (left) pad a centered content row gets inside the body
/// (in columns). Mirrors the three-button dialogs'
/// `pad = (body_w - raw_w) / 2` so the button rects line up with the
/// painted pixels.
pub fn leading_pad(body_w: u16, raw_w: u16) -> u16 {
    if body_w > raw_w {
        (body_w - raw_w) / 2
    } else {
        0
    }
}

/// Per-button rects for the three-button dialog row layout
/// (`create_session_dialog.rs` / `exit_confirmation_dialog.rs` /
/// `board_exit_confirmation_dialog.rs`): a 1-space lead, then ` <label> `
/// buttons separated by a `gap`-column dim gap. `start_x` is the row's
/// left edge INCLUDING any inserted leading pad (body.x + pad). Each
/// button cell is `label.width() + 2` wide (the surrounding spaces).
pub fn three_button_rects(start_x: u16, row_y: u16, labels: &[&str], gap: u16) -> Vec<Rect> {
    let mut x = start_x.saturating_add(1);
    let mut rects = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            x = x.saturating_add(gap);
        }
        let w = label.width() + 2;
        rects.push(Rect {
            x,
            y: row_y,
            width: w as u16,
            height: 1,
        });
        x = x.saturating_add(w as u16);
    }
    rects
}

/// Per-button rects for the separator-style button row used by
/// `ConfirmDialog` / `MergeConfirmDialog` / `SessionWorktreesDialog`:
/// ` {label} ` buttons joined by a `sep_width`-column separator
/// (`FOOTER_SEPARATOR` = `" │ "`), with no lead/trail space. `start_x` is
/// the body's left edge (no leading pad is inserted on this layout).
pub fn separated_button_rects(
    start_x: u16,
    row_y: u16,
    labels: &[&str],
    sep_width: u16,
) -> Vec<Rect> {
    let mut x = start_x;
    let mut rects = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            x = x.saturating_add(sep_width);
        }
        let w = label.width() + 2;
        rects.push(Rect {
            x,
            y: row_y,
            width: w as u16,
            height: 1,
        });
        x = x.saturating_add(w as u16);
    }
    rects
}

/// The natural raw width (columns) of a three-button row: 1-space lead,
/// ` <label> ` buttons, 2-column dim gaps between them, 1-space trail.
/// Used with [`leading_pad`] to center the row exactly as the paint path
/// does.
pub fn three_button_raw_width(labels: &[&str]) -> u16 {
    let buttons: u16 = labels.iter().map(|l| l.width() as u16 + 2).sum();
    let gaps: u16 = (labels.len().saturating_sub(1) * 2 + 2) as u16;
    buttons.saturating_add(gaps)
}

/// Build the [`LastLayout`] for a three-button dialog row. `row_index` is
/// the content-row index the button row is painted at (1 for the
/// three-button dialogs: rows = [description, button_row]). The dialog
/// descriptor is used to resolve the frame rect + content width, so the
/// result matches the painted pixels for the given `area`.
pub fn three_button_layout(
    area: Rect,
    dialog: &FspecDialog<'_>,
    row_index: usize,
    labels: &[&str],
) -> LastLayout {
    let rect = crate::components::dialog_theme::dialog_rect(area, dialog);
    let Some(body) = dialog_body(rect) else {
        return LastLayout::new();
    };
    let footer_h = footer_line_count(dialog.footer);
    let has_query = dialog.query_row.is_some();
    let Some(row_y) = content_row_y(rect, footer_h, has_query, row_index) else {
        return LastLayout {
            dialog_rect: Some(rect),
            controls: Vec::new(),
        };
    };
    // Center the button row within the body — mirrors the paint path's
    // `pad = (body_w - raw_w) / 2`. `body_w` here is the body width,
    // which equals `inner_content_width` (the frame was sized from it).
    let body_w = body.width;
    let raw_w = three_button_raw_width(labels);
    let pad = leading_pad(body_w, raw_w);
    let start_x = body.x.saturating_add(pad);
    LastLayout {
        dialog_rect: Some(rect),
        controls: three_button_rects(start_x, row_y, labels, 2),
    }
}

/// Build the [`LastLayout`] for a separator-style button row
/// (`ConfirmDialog` / `MergeConfirmDialog` / `SessionWorktreesDialog`):
/// ` {label} ` buttons joined by `FOOTER_SEPARATOR` (3 cols), starting at
/// the body's left edge (no leading pad on this layout).
pub fn separated_button_layout(
    area: Rect,
    dialog: &FspecDialog<'_>,
    row_index: usize,
    labels: &[&str],
) -> LastLayout {
    let rect = crate::components::dialog_theme::dialog_rect(area, dialog);
    let Some(body) = dialog_body(rect) else {
        return LastLayout::new();
    };
    let footer_h = footer_line_count(dialog.footer);
    let has_query = dialog.query_row.is_some();
    let Some(row_y) = content_row_y(rect, footer_h, has_query, row_index) else {
        return LastLayout {
            dialog_rect: Some(rect),
            controls: Vec::new(),
        };
    };
    // FOOTER_SEPARATOR = " │ " (space + box-draw + space) = 3 columns.
    LastLayout {
        dialog_rect: Some(rect),
        controls: separated_button_rects(body.x, row_y, labels, 3),
    }
}

/// Build the [`LastLayout`] for a row-list dialog: every content row
/// (0..`row_count`) is clickable across the full body width.
pub fn row_list_layout(area: Rect, dialog: &FspecDialog<'_>, row_count: usize) -> LastLayout {
    let rect = crate::components::dialog_theme::dialog_rect(area, dialog);
    let footer_h = footer_line_count(dialog.footer);
    let has_query = dialog.query_row.is_some();
    let mut controls = Vec::with_capacity(row_count);
    for i in 0..row_count {
        if let Some(rr) = content_row_rect(rect, footer_h, has_query, i) {
            controls.push(rr);
        }
    }
    LastLayout {
        dialog_rect: Some(rect),
        controls,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// A 60x12 dialog with a 1-line footer: spacious, footer shown.
    fn sample() -> (Rect, u16, bool) {
        (
            Rect {
                x: 0,
                y: 0,
                width: 60,
                height: 12,
            },
            1,
            false,
        )
    }

    #[test]
    fn content_row_y_first_row_is_body_top_plus_spacious_gap() {
        let (rect, footer, query) = sample();
        // body.y = 2; spacious → content_start = 2 + 2 = 4; no query row.
        assert_eq!(content_row_y(rect, footer, query, 0), Some(4));
        assert_eq!(content_row_y(rect, footer, query, 1), Some(5));
    }

    #[test]
    fn content_row_y_none_when_clipped_by_footer() {
        // 60x12: body.h = 8. footer_h=1 spacious reserves 2 →
        // body_row_end = body.y + (8 - 2) = 2 + 6 = 8. Content rows 4..7
        // are valid (y < 8); row_index 4 → y=8 → clipped.
        let (rect, footer, query) = sample();
        assert_eq!(content_row_y(rect, footer, query, 3), Some(7));
        assert_eq!(content_row_y(rect, footer, query, 4), None);
    }

    #[test]
    fn content_row_y_accounts_for_query_row() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 60,
            height: 12,
        };
        // has_query_row → first content row is pushed down by one.
        assert_eq!(content_row_y(rect, 0, true, 0), Some(5));
        assert_eq!(content_row_y(rect, 0, false, 0), Some(4));
    }

    #[test]
    fn content_row_rect_is_full_body_width() {
        let (rect, footer, query) = sample();
        let rr = content_row_rect(rect, footer, query, 0).expect("row 0");
        assert_eq!(rr.x, 2);
        assert_eq!(rr.width, 56);
        assert_eq!(rr.height, 1);
    }

    #[test]
    fn dialog_body_is_none_when_too_small() {
        assert!(dialog_body(Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3
        })
        .is_none());
        let b = dialog_body(Rect {
            x: 10,
            y: 20,
            width: 12,
            height: 12,
        })
        .expect("body");
        assert_eq!(b.x, 12);
        assert_eq!(b.y, 22);
        assert_eq!(b.width, 8);
        assert_eq!(b.height, 8);
    }

    #[test]
    fn hit_button_selects_the_containing_rect_and_misses_the_gap() {
        // Two buttons with a 2-column gap between them.
        let rects = vec![
            Rect {
                x: 10,
                y: 4,
                width: 4,
                height: 1,
            },
            Rect {
                x: 18,
                y: 4,
                width: 4,
                height: 1,
            },
        ];
        assert_eq!(hit_button(&rects, 10, 4), Some(0));
        assert_eq!(hit_button(&rects, 13, 4), Some(0));
        assert_eq!(hit_button(&rects, 18, 4), Some(1));
        // The gap (cols 14..17) and everything off the row miss.
        assert_eq!(hit_button(&rects, 15, 4), None);
        assert_eq!(hit_button(&rects, 11, 5), None);
        assert_eq!(hit_button(&rects, 5, 4), None);
    }

    #[test]
    fn footer_line_count_counts_lines_and_empty_is_zero() {
        assert_eq!(footer_line_count(""), 0);
        assert_eq!(footer_line_count("a b c"), 1);
        assert_eq!(footer_line_count("one\ntwo\nthree"), 3);
    }

    #[test]
    fn three_button_rects_match_the_painted_layout() {
        // ` Yes ` (5), 2-gap, ` Yes - Isolated ` (16), 2-gap, ` Cancel ` (8).
        let rects = three_button_rects(20, 4, &["Yes", "Yes - Isolated", "Cancel"], 2);
        assert_eq!(rects.len(), 3);
        // Lead space: first button starts at start_x + 1.
        assert_eq!(rects[0].x, 21);
        assert_eq!(rects[0].width, 5);
        assert_eq!(rects[1].x, 21 + 5 + 2);
        assert_eq!(rects[1].width, 16);
        assert_eq!(rects[2].x, rects[1].x + 16 + 2);
        assert_eq!(rects[2].width, 8);
        assert!(rects.iter().all(|r| r.y == 4 && r.height == 1));
    }

    #[test]
    fn separated_button_rects_use_the_separator_width_as_gap() {
        // ` Merge ` (7), sep 3, ` Discard ` (9), sep 3, ` Cancel ` (8).
        let rects = separated_button_rects(10, 6, &["Merge", "Discard", "Cancel"], 3);
        assert_eq!(rects[0].x, 10);
        assert_eq!(rects[0].width, 7);
        assert_eq!(rects[1].x, 10 + 7 + 3);
        assert_eq!(rects[1].width, 9);
        assert_eq!(rects[2].x, 10 + 7 + 3 + 9 + 3);
        assert_eq!(rects[2].width, 8);
    }

    #[test]
    fn leading_pad_centers_the_row_when_the_body_is_wider() {
        assert_eq!(leading_pad(60, 30), 15);
        assert_eq!(leading_pad(30, 30), 0);
        // Body narrower than the row: no negative pad.
        assert_eq!(leading_pad(10, 30), 0);
    }

    #[test]
    fn last_layout_hit_and_contains() {
        let layout = LastLayout {
            dialog_rect: Some(Rect {
                x: 0,
                y: 0,
                width: 60,
                height: 12,
            }),
            controls: vec![Rect {
                x: 20,
                y: 4,
                width: 8,
                height: 1,
            }],
        };
        assert!(layout.contains(5, 5));
        assert!(!layout.contains(70, 5));
        assert_eq!(layout.hit(21, 4), Some(0));
        assert_eq!(layout.hit(20, 5), None);
    }

    #[test]
    fn last_layout_is_empty_before_first_render() {
        let layout = LastLayout::new();
        assert!(!layout.contains(10, 10));
        assert_eq!(layout.hit(10, 10), None);
    }
}
