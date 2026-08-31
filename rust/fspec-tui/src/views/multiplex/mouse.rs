//! MUX-001/BUG-166 — mux mouse routing: hit-test, click-to-focus,
//! per-divider drag.
//!
//! Feature: spec/features/mux-dividers-percentage-scale.feature
//!
//! Mouse events hit-test the dividers first (one per inter-pane gap —
//! first match wins), then the pane rects (click-to-focus + forward to
//! that pane's handler). Clicks in gaps (footer row, outside every rect)
//! are ignored and never move focus.
//!
//! This module classifies mouse events into [`MouseDecision`]s; the
//! Navigator executes them (keeps the layout borrow and the
//! pane-handler borrow disjoint).

use crossterm::event::{Event, MouseEventKind};

use super::MultiplexLayout;

/// What the mux layer wants the Navigator to do with a mouse event.
pub enum MouseDecision {
    /// Divider mouse-down: begin a drag + focus the divider.
    DividerDown { index: usize },
    /// Divider drag: move the split to the cursor position.
    DividerDrag { index: usize },
    /// Divider release: end the drag.
    DividerUp { index: usize },
    /// Click inside a pane: focus that pane, then forward the event.
    Pane { index: usize },
    /// Gap (footer / outside every rect): ignore, focus unchanged.
    Gap,
}

/// The index of the divider whose rect contains `(col, row)` (first
/// match wins — the dividers are disjoint by construction).
fn divider_hit(layout: &MultiplexLayout, col: u16, row: u16) -> Option<usize> {
    layout
        .divider_rects
        .iter()
        .position(|d| crate::mouse::hit_test::rect_contains(*d, col, row))
}

/// Classify a mouse event against the current mux layout.
pub fn classify_mouse(layout: &MultiplexLayout, event: &Event) -> MouseDecision {
    if !layout.config.enabled {
        return MouseDecision::Gap;
    }
    let Event::Mouse(m) = event else {
        return MouseDecision::Gap;
    };
    let m = *m;

    // While a divider drag is in flight, Drag/Up keep routing to the
    // DRAGGED divider regardless of where the cursor has moved (the
    // cursor leaves the 1-col divider as soon as it starts tracking).
    if layout.is_dragging {
        let Some(index) = layout.drag_index else {
            return MouseDecision::Gap;
        };
        return match m.kind {
            MouseEventKind::Drag(_) => MouseDecision::DividerDrag { index },
            MouseEventKind::Up(_) => MouseDecision::DividerUp { index },
            _ => MouseDecision::Gap,
        };
    }

    // Divider hit-test first (R4/BUG-166 drag state machine — every
    // inter-pane gap has its own divider).
    if let Some(index) = divider_hit(layout, m.column, m.row) {
        return match m.kind {
            MouseEventKind::Down(_) => MouseDecision::DividerDown { index },
            MouseEventKind::Drag(_) => MouseDecision::DividerDrag { index },
            MouseEventKind::Up(_) => MouseDecision::DividerUp { index },
            _ => MouseDecision::Gap,
        };
    }

    // Pane hit-test: click-to-focus (R2), then forward to that pane.
    for (i, rect) in layout.pane_rects().iter().enumerate() {
        if crate::mouse::hit_test::rect_contains(*rect, m.column, m.row) {
            if matches!(m.kind, MouseEventKind::Down(_)) {
                return MouseDecision::Pane { index: i };
            }
            // Non-click mouse over a pane (wheel, drag): forward to the
            // pane that currently has focus.
            return MouseDecision::Pane {
                index: layout.focus(),
            };
        }
    }

    MouseDecision::Gap
}

/// The cursor position (col, row) of a mouse event.
pub fn mouse_pos(event: &Event) -> Option<(u16, u16)> {
    match event {
        Event::Mouse(m) => Some((m.column, m.row)),
        _ => None,
    }
}
