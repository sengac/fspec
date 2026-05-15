//! Rectangle hit-testing for mouse events.
//!
//! Feature: spec/features/rpc023-source-shape.feature
//! Card: RPC-023.
//!
//! Components that want to consume mouse events remember their
//! last-rendered [`Rect`] in a `Cell<Option<Rect>>` field and call
//! [`rect_contains`] before consuming the event so wheel scroll on a
//! non-targeted region falls through to the next layer.
//!
//! The TS reference (`src/tui/utils/mouseProtocol.ts`) did NOT have a
//! hit-test helper because Ink exposed only raw escape strings and
//! every consumer rolled its own column-band math. crossterm's
//! `Event::Mouse(MouseEvent { column, row, .. })` gives us pre-parsed
//! pixel coordinates, so a single shared helper is enough for every
//! follow-up slice (RPC-019 VirtualList, RPC-020 slash-command popup,
//! RPC-022 modal-dialog drag).

use ratatui::layout::Rect;

/// True iff `(x, y)` lies inside `rect` using half-open semantics on
/// the right/bottom edges — matches ratatui's [`Rect::intersects`]
/// convention.
///
/// Half-open means a point at `(rect.x + rect.width, rect.y)` is
/// considered OUTSIDE the rectangle (it would belong to the next
/// horizontal cell). Same for the bottom edge.
#[inline]
pub fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_interior_and_top_left_corner() {
        let r = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };
        assert!(rect_contains(r, 5, 5), "(5,5) is the top-left corner");
        assert!(rect_contains(r, 14, 14), "(14,14) is the bottom-right interior cell");
    }

    #[test]
    fn rejects_points_past_the_right_or_bottom_edge() {
        let r = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };
        assert!(!rect_contains(r, 15, 14), "(15,14) is one past the right edge");
        assert!(!rect_contains(r, 14, 15), "(14,15) is one past the bottom edge");
    }

    #[test]
    fn rejects_points_before_the_top_or_left_edge() {
        let r = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };
        assert!(!rect_contains(r, 4, 5), "(4,5) is before the left edge");
        assert!(!rect_contains(r, 5, 4), "(5,4) is before the top edge");
    }

    #[test]
    fn zero_width_or_height_rectangles_never_contain_anything() {
        let r0 = Rect {
            x: 5,
            y: 5,
            width: 0,
            height: 10,
        };
        let r1 = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 0,
        };
        assert!(!rect_contains(r0, 5, 5));
        assert!(!rect_contains(r1, 5, 5));
    }
}
