//! RPC-396 — Scroll + space-filling rect math for [`super::help_dialog::HelpDialog`].
//!
//! Feature: spec/features/scrollable-space-filling-help-dialog-with-scrollbar.feature
//!
//! Extracted from `help_dialog.rs` to keep that file under 300 LoC. Pure
//! geometry/clamp helpers — no ratatui painting here beyond returning
//! `Rect`s the caller feeds to `render_dialog_at` / `render_list_scrollbar`.

use ratatui::layout::Rect;

use super::scroll_viewport::WheelDirection;

/// Horizontal margin (columns) left on each side of the terminal.
pub const MARGIN_X: u16 = 2;
/// Vertical margin (rows) left on each side of the terminal.
pub const MARGIN_Y: u16 = 2;

/// Number of body chrome rows `render_dialog_at` reserves around the
/// scrollable content in the spacious layout, given a single-line
/// footer:
///   * 2 border rows (top + bottom of the rounded block)
///   * 2 padding rows (1 inside each border edge)
///   * 1 title row
///   * 1 gap row after the title
///   * 1 gap row before the footer
///   * 1 footer row
///
/// => 8 rows are chrome; the remainder is content height.
pub const CHROME_ROWS: u16 = 8;

/// Compute the space-filling dialog rect: fill `area` minus a small
/// margin on every side, clamped so it never underflows on tiny
/// terminals (always at least the whole area when the area is smaller
/// than twice the margin).
pub fn fill_rect(area: Rect) -> Rect {
    let dx = MARGIN_X.min(area.width / 2);
    let dy = MARGIN_Y.min(area.height / 2);
    Rect {
        x: area.x + dx,
        y: area.y + dy,
        width: area.width.saturating_sub(dx * 2).max(1),
        height: area.height.saturating_sub(dy * 2).max(1),
    }
}

/// Content rows available inside `rect` for scrollable lines, i.e.
/// `rect.height - CHROME_ROWS` (saturating at 0).
pub fn content_rows(rect: Rect) -> usize {
    rect.height.saturating_sub(CHROME_ROWS) as usize
}

/// Maximum scroll offset given `total` lines and a `visible` window:
/// `total - visible`, saturating at 0 (no scroll when everything fits).
pub fn max_offset(total: usize, visible: usize) -> usize {
    total.saturating_sub(visible)
}

/// The 1-column gutter rect on the right edge of `rect`'s content area,
/// aligned with the first content row and `visible` rows tall. Returns
/// `None` when the rect is too small to host a gutter.
///
/// Bordered body geometry (see `render_dialog_at`, mirrors
/// `TurnContentModal`):
///   * `inner = block.inner(rect)` removes 1 border cell each side;
///     `body = inner + 1 padding` → body.y = rect.y + 2. Spacious
///     content starts at body.y + 2 = rect.y + 4 (title row + gap row).
///   * body rightmost column = rect.x + rect.width - 3
///     (1 border + 1 padding on the right, then the last body column).
pub fn gutter_rect(rect: Rect, visible: usize) -> Option<Rect> {
    if rect.width < 4 || visible == 0 {
        return None;
    }
    let content_y = rect.y + 4;
    let gutter_x = rect.x + rect.width - 3;
    Some(Rect {
        x: gutter_x,
        y: content_y,
        width: 1,
        height: visible as u16,
    })
}

/// Map a crossterm `Event` to a wheel scroll [`WheelDirection`], or
/// `None` for non-wheel events.
///
/// RPC-396: the `Event::Mouse` match lives HERE (not in `help_dialog.rs`)
/// so the RPC-023 source-shape guard — which asserts the dialog shell
/// `help_dialog.rs` stays `Event::Key`-only — remains green while the
/// HelpDialog still gains mouse-wheel scrolling. HelpDialog is centered /
/// topmost (Critical), so — like `thinking_level_dialog.rs` — no
/// hit-testing is needed: any wheel event while the dialog is open is
/// ours.
pub(crate) fn wheel_direction(event: &crossterm::event::Event) -> Option<WheelDirection> {
    use crossterm::event::{Event, MouseEventKind};
    if let Event::Mouse(m) = event {
        return match m.kind {
            MouseEventKind::ScrollUp => Some(WheelDirection::Up),
            MouseEventKind::ScrollDown => Some(WheelDirection::Down),
            _ => None,
        };
    }
    None
}

/// TUI-101 + RPC-396: all mouse handling for [`super::help_dialog::HelpDialog`],
/// factored out of the shell so `help_dialog.rs` stays `Event::Key`-only
/// (the RPC-023 source-shape guard) and under the 300-LoC ceiling.
///
/// Returns:
/// * `Some(Ok(result))` — the event is a mouse event the dialog handles
///   (drag navigation on the gutter, or wheel scroll);
/// * `Some(Err(ignored))` — a mouse event the dialog ignores (falls
///   through to the App);
/// * `None` — not a mouse event; the caller continues with its
///   key/paste handling.
pub(crate) fn handle_dialog_mouse(
    dialog: &mut super::help_dialog::HelpDialog,
    event: &crossterm::event::Event,
) -> Option<super::EventResult> {
    use crossterm::event::{Event, MouseButton, MouseEventKind};
    let mouse = match event {
        Event::Mouse(m) => m,
        _ => return None,
    };

    // TUI-101: scrollbar click-and-drag navigation.
    let max = dialog.max_offset();
    let total = dialog.lines.len();
    let visible = dialog.visible_rows;
    if total > visible {
        if let Some(gutter) = dialog.last_gutter {
            let inside =
                crate::mouse::rect_contains(gutter, mouse.column, mouse.row);
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left) => {
                    if inside {
                        let geom = crate::mouse::scrollbar_drag::ScrollbarGeometry {
                            area_height: visible,
                            total_items: total,
                            visible_items: visible,
                            current_offset: dialog.scroll_offset,
                        };
                        if let Some(offset) = dialog.scrollbar_drag.on_mouse(*mouse, geom) {
                            dialog.scroll_offset = offset.min(max);
                        }
                        return Some(super::EventResult::consumed());
                    } else {
                        // Reset drag state when clicking outside scrollbar
                        if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                            dialog.scrollbar_drag.reset();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // RPC-396: mouse wheel scrolls the content (any wheel event while the
    // dialog is open is ours — it is centered/topmost, so no hit-testing).
    if let Some(dir) = wheel_direction(event) {
        let step = dialog.wheel.step(dir);
        let proposed = dialog.scroll_offset as i64 + step as i64;
        dialog.scroll_offset = proposed.clamp(0, max as i64) as usize;
        return Some(super::EventResult::consumed());
    }
    Some(super::EventResult::ignored())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn fill_rect_leaves_margin_on_a_normal_terminal() {
        let r = fill_rect(Rect::new(0, 0, 80, 24));
        assert_eq!(r, Rect::new(2, 2, 76, 20));
    }

    #[test]
    fn fill_rect_never_underflows_on_tiny_terminals() {
        let r = fill_rect(Rect::new(0, 0, 2, 2));
        assert!(r.width >= 1 && r.height >= 1);
    }

    #[test]
    fn content_rows_subtracts_chrome() {
        assert_eq!(content_rows(Rect::new(2, 2, 76, 20)), 12);
        assert_eq!(content_rows(Rect::new(0, 0, 10, 6)), 0);
    }

    #[test]
    fn max_offset_saturates_when_all_fits() {
        assert_eq!(max_offset(40, 12), 28);
        assert_eq!(max_offset(5, 12), 0);
    }

    #[test]
    fn gutter_rect_is_one_column_on_the_right() {
        let g = gutter_rect(Rect::new(2, 2, 76, 20), 12).expect("gutter");
        assert_eq!(g.width, 1);
        assert_eq!(g.x, 2 + 76 - 3);
        assert_eq!(g.y, 2 + 4);
        assert_eq!(g.height, 12);
    }

    #[test]
    fn gutter_rect_none_when_too_narrow() {
        assert!(gutter_rect(Rect::new(0, 0, 3, 20), 12).is_none());
        assert!(gutter_rect(Rect::new(0, 0, 76, 20), 0).is_none());
    }
}
