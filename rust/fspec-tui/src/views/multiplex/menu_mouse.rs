//! MENU-004 R-MOUSE — the mux top-row bar's mouse hit-test arms.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Card: R-MOUSE.
//!
//! The bar row is hit-tested BEFORE the dividers/panes (the bar is the
//! only bar on screen in mux — the per-pane bars are suppressed,
//! R-SUPPRESS). The cached per-frame geometry comes from the MUX layer
//! (`menu_item_rects` / `menu_cells` / `menu_open_panel`, RED CARD 1) —
//! `None` / `menu_bar_painted == false` keeps every arm inert (the
//! 3-row degradation, the pre-first-render frame).
//!
//! Arms (mirror of `views/board/menu_mouse.rs`, R7 parity):
//! - **Left click on a Zone A item** — focus + toggle its dropdown
//!   (the already-open item closes, GUI parity).
//! - **Left click on a Zone B cell** — a chip activates its session
//!   (NO view flip, R-CHIPS); a pane view label focuses that pane
//!   (R-ZONEB).
//! - **Left click on EMPTY bar-row space** — while the bar is engaged
//!   (ring focus or open dropdown) it is the 'leave the bar' gesture:
//!   the `MenuDismissBar` 'close + de-select' token (BUG-197 R2) —
//!   otherwise inert (no pane focus change).
//! - **Left click INSIDE an open dropdown's panel** — a hit on an
//!   entry row executes THAT row on the same event (BUG-196 R2, GUI
//!   parity with Enter); a hit on the panel's border/ellipsis rows is
//!   swallowed (the panel owns its rect — no fall-through to the
//!   pane beneath).
//! - **Left click OUTSIDE an open dropdown** (off the bar) — close the
//!   dropdown AND clear the bar's ring focus (BUG-196 R3: the
//!   `MenuDismissBar` 'close + de-select' gesture) AND let the click
//!   land on the pane (R-MOUSE: "the click still lands").
//! - **Wheel ScrollLeft/ScrollRight over the bar row** — walk the ring
//!   like the keys (only when the bar has a focus — otherwise ignore).
//! - **Wheel ScrollUp/ScrollDown over the open dropdown's panel** —
//!   move the cursor (R6 parity).
//!
//! All claims emit `Action`s the App's `dispatch_menu` Mux branch
//! mutates (the single-mutation-surface pattern).

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};

use crate::components::Action;
use crate::mouse::rect_contains;

use super::menu_keys::ZoneBTarget;
use super::MultiplexLayout;

/// What the mux bar wants the Navigator to do with a mouse event.
pub(crate) enum MuxBarMouseDecision {
    /// The bar claimed the event and emits this `Action` (the App's Mux
    /// branch mutates the layout's bar state). Boxed — see
    /// `BlocklistEvent::Emit` for the `large_enum_variant` rationale.
    Claimed(Box<Action>),
    /// The bar claimed the event and swallowed it (no action) — a bar-row
    /// click that hit no item/cell.
    Swallowed,
    /// The dropdown closed on this outside click; the click must STILL
    /// land on the pane it hit (the Navigator focuses + forwards it).
    CloseOutsideThenLand,
    /// The bar owns this event but no arm applies — ignore it entirely
    /// (e.g. wheel over the bar with no bar focus and no open dropdown).
    Ignore,
    /// Not a bar event — the dividers/panes keep their usual routing.
    Pass,
}

/// Box the action for the `Claimed` variant (the `large_enum_variant`
/// shape — `Action` is ≥ 216 bytes, see `BlocklistEvent::Emit`).
fn claim(action: Action) -> MuxBarMouseDecision {
    MuxBarMouseDecision::Claimed(Box::new(action))
}

/// The bar row's y (row 0 of the mux area — derived from the pane
/// rects the same frame's render cached; `None` before the first
/// render or on the 3-row degradation).
fn bar_row(layout: &MultiplexLayout) -> Option<u16> {
    layout.pane_rects().first().map(|r| r.y.saturating_sub(1))
}

/// Classify a mouse event against the cached mux bar geometry.
pub(crate) fn classify_bar_mouse(layout: &MultiplexLayout, event: &Event) -> MuxBarMouseDecision {
    let MouseEvent {
        kind, column, row, ..
    } = match event {
        Event::Mouse(m) => *m,
        _ => return MuxBarMouseDecision::Pass,
    };
    // Inert unless the last frame painted the bar + cached its geometry.
    if !layout.menu_bar_painted() {
        return MuxBarMouseDecision::Pass;
    }
    let Some(bar_y) = bar_row(layout) else {
        return MuxBarMouseDecision::Pass;
    };

    let in_bar_row = row == bar_y;

    // ── wheel ────────────────────────────────────────────────────────────
    if matches!(
        kind,
        MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight
            | MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
    ) {
        if in_bar_row {
            // R-MOUSE: wheel L/R over the bar walks the ring — but ONLY
            // when the bar's ring is active (a focused item/cell or an
            // open dropdown). With no bar focus and no open dropdown
            // the event is IGNORED (the bar is inert — R-MOUSE: "wheel
            // over the bar with no focus and no open dropdown is
            // ignored"). Up/Down on the bar row itself has no target
            // (the open panel starts one row below the bar).
            if layout.menu_ring_active()
                && matches!(
                    kind,
                    MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight
                )
            {
                let delta = match kind {
                    MouseEventKind::ScrollLeft => -1,
                    MouseEventKind::ScrollRight => 1,
                    _ => unreachable!("guard above"),
                };
                return claim(Action::MenuMove(delta));
            }
            return MuxBarMouseDecision::Ignore;
        }
        // Wheel Up/Down over the OPEN dropdown's panel moves the cursor
        // (R6 parity with the keys' Up/Down cursor walk) — the panel
        // starts one row below the bar, so this arm never double-fires
        // with the bar-row arm above.
        if let Some(panel) = layout.menu_open_panel() {
            if rect_contains(panel, column, row) {
                return match kind {
                    MouseEventKind::ScrollUp => claim(Action::MenuDropdownCursor(-1)),
                    MouseEventKind::ScrollDown => claim(Action::MenuDropdownCursor(1)),
                    _ => MuxBarMouseDecision::Pass,
                };
            }
        }
        // Wheel off the bar row and off the panel belongs to the pane
        // beneath it (e.g. the board pane's own ring walk).
        return MuxBarMouseDecision::Pass;
    }

    // ── left click ──────────────────────────────────────────────────────
    if !matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
        return MuxBarMouseDecision::Pass;
    }

    if in_bar_row {
        if let Some(items) = layout.menu_item_rects() {
            for (idx, rect) in items.iter().enumerate() {
                if rect_contains(*rect, column, row) {
                    // R-MOUSE: a click focuses the item AND toggles its
                    // dropdown in one gesture (the App's Mux branch runs
                    // `menu_open_or_close` — a re-click on the open item
                    // closes it, GUI parity).
                    return claim(Action::MenuOpenDropdown(idx));
                }
            }
        }
        if let Some(cells) = layout.menu_cells() {
            for (cell, rect) in cells.iter() {
                if !rect_contains(*rect, column, row) {
                    continue;
                }
                // Every DisplayCell variant carries the ORIGINAL zone_b
                // index it covers (folds map to the first folded cell)
                // — the target resolves from that original index.
                let orig = match cell {
                    crate::components::menu_bar::DisplayCell::View { orig, .. }
                    | crate::components::menu_bar::DisplayCell::Chip { orig } => *orig,
                    crate::components::menu_bar::DisplayCell::Fold { orig, .. } => *orig,
                };
                // A click NEVER changes pane focus via the bar itself
                // (the bar is an overlay control, not a pane) — but a
                // view-label click focuses that pane (R-ZONEB).
                return match layout.zone_b_target(orig) {
                    ZoneBTarget::Chip(i) => claim(Action::MenuChipActivate(i)),
                    ZoneBTarget::Pane(pane) => claim(Action::MenuFocusPane(pane)),
                    ZoneBTarget::None => MuxBarMouseDecision::Swallowed,
                };
            }
        }
        // A bar-row click that hit no item/cell (empty bar space).
        // BUG-197 R2: while the bar is engaged (a ring focus or an open
        // dropdown) it is the 'leave the bar' gesture — the `MenuDismissBar`
        // 'close + de-select' token clears the bar's ring focus without
        // any pane focus change (the bar owns its row, no fall-through).
        // With nothing engaged it stays inert (Swallowed).
        if layout.menu_ring_active() {
            return claim(Action::MenuDismissBar);
        }
        return MuxBarMouseDecision::Swallowed;
    }

    // Off the bar row: a left click INSIDE the open dropdown's panel —
    // a hit on an entry row executes THAT row on the same event (BUG-
    // 196 R2, GUI parity with Enter — the shared `dropdown_row_at`
    // hit-test reuses the painter's scroll-window math); a hit on the
    // panel's border/ellipsis rows is swallowed (the panel owns its
    // rect — the click must NOT fall through to the pane beneath).
    if let Some((category, cursor)) = layout.open_menu() {
        if let Some(panel) = layout.menu_open_panel() {
            if rect_contains(panel, column, row) {
                if let Some(row) = crate::components::menu_bar::dropdown_row_at(
                    panel, category, cursor, column, row,
                ) {
                    return claim(Action::MenuExecuteItem { category, row });
                }
                return MuxBarMouseDecision::Swallowed;
            }
        }
    }

    // Off the bar row: a left click OUTSIDE the open dropdown closes it
    // AND clears the bar's ring focus (BUG-196 R3: the `MenuDismissBar`
    // 'close + de-select' gesture) — the click still lands on the pane
    // (the Navigator focuses + forwards on the same event). BUG-197 R1
    // extends this to EVERY bar focus state: with the dropdown CLOSED
    // but a ring focus present, the click away must still de-select.
    let outside_panel = layout
        .menu_open_panel()
        .is_none_or(|panel| !rect_contains(panel, column, row));
    let engaged = layout.open_menu().is_some() || layout.menu_focus().is_some();
    if engaged && outside_panel {
        return MuxBarMouseDecision::CloseOutsideThenLand;
    }
    MuxBarMouseDecision::Pass
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::MuxPaneKind;

    fn enabled_layout(panes: Vec<MuxPaneKind>) -> MultiplexLayout {
        let mut m = MultiplexLayout::new();
        m.enable_default();
        m.set_pane_list(panes, None);
        m
    }

    #[test]
    fn inert_until_the_bar_paints() {
        // @step Given a mux layout that never painted the bar
        let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
        // The pane rects exist but NO paint ever ran — the painted
        // flag is off, so every arm stays inert regardless of geometry.
        m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
        m.menu_item_rects = None;
        // @step When a mouse event is classified
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollRight,
            column: 5,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        // @step Then the arms stay inert (no paint → no geometry → Pass)
        assert!(matches!(
            classify_bar_mouse(&m, &event),
            MuxBarMouseDecision::Pass
        ));
    }

    #[test]
    fn wheel_over_the_bar_with_no_focus_is_ignored() {
        // @step Given a painted bar with item + cell geometry but no focus
        let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
        m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
        m.menu_bar_painted = true;
        m.menu_item_rects = Some(vec![
            ratatui::layout::Rect::new(1, 0, 7, 1),
            ratatui::layout::Rect::new(9, 0, 4, 1),
        ]);
        m.menu_cells = Some(Vec::new());
        m.menu_open_panel = None;
        m.menu_focus = None;
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollRight,
            column: 5,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        // @step When wheel right is sent over the bar row
        // @step Then the event is ignored (no focus, no open dropdown)
        assert!(matches!(
            classify_bar_mouse(&m, &event),
            MuxBarMouseDecision::Ignore
        ));
    }

    #[test]
    fn click_outside_an_open_dropdown_closes_and_lands() {
        // @step Given a painted bar with an open dropdown panel
        let mut m = enabled_layout(vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
        m.pane_rects.push(ratatui::layout::Rect::new(0, 1, 120, 20));
        m.menu_bar_painted = true;
        m.menu_item_rects = Some(vec![
            ratatui::layout::Rect::new(1, 0, 7, 1),
            ratatui::layout::Rect::new(9, 0, 4, 1),
        ]);
        m.menu_cells = Some(Vec::new());
        m.menu_open_panel = Some(ratatui::layout::Rect::new(1, 1, 30, 9));
        m.menu_focus = Some(crate::components::menu_bar::MenuFocus::Item(0));
        m.open_menu = Some((0, 0));
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 60,
            row: 10, // inside a pane, outside the panel
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        // @step When a left click lands on a pane outside the panel
        // @step Then the dropdown closes AND the click still lands
        assert!(matches!(
            classify_bar_mouse(&m, &event),
            MuxBarMouseDecision::CloseOutsideThenLand
        ));
    }
}
