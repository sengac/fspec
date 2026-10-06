//! MENU-002 — the board's 2-zone menu bar + dropdown mouse arms.
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//! Card: MENU-002 (R5, R7, R9).
//!
//! Extracted from `mouse.rs` so both files stay under the 300-LoC
//! ceiling. Hit-test order (R7: the bar rect runs BEFORE the
//! details-strip and content tests):
//!
//! 1. **A left click on the bar row** — a menu item focuses + opens
//!    its dropdown (one gesture; the already-open item closes — GUI
//!    parity, R7), a chip activates (R5, same as Enter on the chip).
//! 2. **A left click INSIDE an open dropdown's panel** — a hit on an
//!    entry row executes THAT row on the same event (BUG-196 R2, GUI
//!    parity with Enter); a hit on the panel's border/ellipsis rows is
//!    swallowed (the panel owns its rect — no fall-through).
//! 3. **A left click OUTSIDE an open dropdown** — the dropdown closes
//!    AND the bar's ring focus clears (BUG-196 R3: the `MenuDismissBar`
//!    'close + de-select' gesture) and the click STILL lands: the
//!    caller's normal header / content selection runs on the same event.
//! 4. **Wheel over the bar row** — `ScrollLeft/ScrollRight` walk the
//!    ring exactly like the keys (R2/R7, dropdown open or closed);
//!    `ScrollUp/ScrollDown` over the OPEN dropdown's panel moves the
//!    cursor (R6). Wheel over the bar with the dropdown closed and no
//!    ring direction is ignored.
//!
//! The render path caches the per-frame [`MenuLayout`] (+ its bar row
//! y + the open panel rect, if any) in `Cell<Option<...>>` fields on
//! [`BoardView`]; this module reads them back.

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};

use crate::components::menu_bar::{dropdown_row_at, MenuLayout};
use crate::components::{Action, EventResult};
use crate::mouse::rect_contains;
use crate::store::BoardStore;

use super::BoardView;

/// The cached per-frame menu-bar geometry (MENU-002 R7 hit-testing).
#[derive(Debug, Clone)]
pub(crate) struct MenuBarGeometry {
    /// The bar row's y (the header's row 3).
    pub bar_y: u16,
    /// The per-frame [`MenuLayout`] (item + cell rects).
    pub layout: MenuLayout,
    /// The open dropdown's panel rect (if any) — wheel cursor target.
    pub open_panel: Option<ratatui::layout::Rect>,
}

impl BoardView {
    /// MENU-002: stash this frame's menu-bar geometry for the mouse
    /// arms (`handle_menu_mouse`). `None` when the bar painted nothing.
    pub(super) fn cache_menu_geometry(&self, geometry: Option<MenuBarGeometry>) {
        *self.last_menu_bar_geometry.borrow_mut() = geometry;
    }
}

/// Route a mouse event against the cached bar geometry. Returns
/// `Some(result)` when a menu arm claimed (or partially claimed) the
/// event: `Consumed` for bar-row / panel hits, `Ignored` when nothing
/// matched. The OUTSIDE-click dropdown close (R7) is emitted INLINE
/// here before the `Some`/`None` decision so the caller's normal
/// selection logic still runs on the same event.
pub(super) fn handle_menu_mouse(
    view: &BoardView,
    event: &Event,
    store: &BoardStore,
) -> Option<EventResult> {
    let mouse_event = match event {
        Event::Mouse(m) => *m,
        _ => return None,
    };
    let guard = view.last_menu_bar_geometry.borrow();
    let geometry = guard.as_ref()?;
    let MouseEvent {
        kind, column, row, ..
    } = mouse_event;
    let in_bar_row = row == geometry.bar_y;

    // BUG-196 R3 / BUG-197 R1: a left click OUTSIDE the panel (and off
    // the bar row) is the 'leave the bar' gesture — the `MenuDismissBar`
    // token expresses 'close + de-select' as one intent. BUG-196 gated
    // this on an OPEN dropdown; BUG-197 R1 extends it to EVERY bar
    // focus state: with the dropdown closed but a ring focus present
    // (item or chip), the click away must still de-select — the bar's
    // ring focus clears, so the surface's key bindings are live again
    // and no stale chip highlight lingers. The click itself still
    // works: the caller's normal selection runs on the same event.
    // (Bar-row clicks are handled by the arms below — item/cell hits
    // and the empty-space de-select.)
    let close_outside = !in_bar_row
        && matches!(kind, MouseEventKind::Down(MouseButton::Left))
        && (store.open_menu().is_some() || store.menu_focus().is_some())
        && geometry
            .open_panel
            .is_none_or(|panel| !rect_contains(panel, column, row))
        && !geometry
            .layout
            .item_rects
            .iter()
            .any(|r| rect_contains(*r, column, row));
    if close_outside {
        view.emit(Action::MenuDismissBar);
    }

    if in_bar_row {
        match kind {
            // R7: wheel L/R over the bar row walks the ring — dropdown
            // open OR closed (R2 parity with the keys).
            MouseEventKind::ScrollLeft => {
                view.clear_details_selection();
                view.emit(Action::MenuMove(-1));
                return Some(EventResult::consumed());
            }
            MouseEventKind::ScrollRight => {
                view.clear_details_selection();
                view.emit(Action::MenuMove(1));
                return Some(EventResult::consumed());
            }
            // R7: a menu item focuses + toggles its dropdown; a chip
            // activates. (An outside left click already closed the
            // dropdown above — a second open-tick on the open item
            // would re-open it, so item hits skip the close branch.)
            MouseEventKind::Down(MouseButton::Left) => {
                for (idx, rect) in geometry.layout.item_rects.iter().enumerate() {
                    if rect_contains(*rect, column, row) {
                        view.emit(Action::MenuMoveToItem(idx));
                        view.emit(Action::MenuOpenDropdown(idx));
                        return Some(EventResult::consumed());
                    }
                }
                for (idx, rect) in geometry.layout.cell_rects.iter().enumerate() {
                    if rect_contains(*rect, column, row) {
                        view.emit(Action::MenuChipActivate(idx));
                        return Some(EventResult::consumed());
                    }
                }
                // BUG-197 R2: a bar-row click that hits NO zone (empty
                // bar space) is the 'leave the bar' gesture — it
                // de-selects the bar (the ring focus clears, so the
                // surface's key bindings are live again) without
                // changing the selection. The bar owns its row, so the
                // click is otherwise inert (no fall-through).
                if store.menu_focus().is_some() || store.open_menu().is_some() {
                    view.emit(Action::MenuDismissBar);
                }
                return Some(EventResult::consumed());
            }
            _ => return None,
        }
    }

    // BUG-196 R1/R2: a left click INSIDE the open dropdown's panel — a
    // hit on an entry row executes THAT row on the same event (GUI menu
    // parity with Enter — the shared `dropdown_row_at` hit-test reuses
    // the painter's scroll-window math); a hit on the panel's
    // border/ellipsis rows is swallowed (the panel owns its rect — no
    // fall-through to the surface behind).
    if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
        if let Some((category, cursor)) = store.open_menu() {
            if let Some(panel) = geometry.open_panel {
                if rect_contains(panel, column, row) {
                    if let Some(row) = dropdown_row_at(panel, category, cursor, column, row) {
                        view.emit(Action::MenuExecuteItem { category, row });
                    }
                    return Some(EventResult::consumed());
                }
            }
        }
    }

    // R6: wheel up/down over the OPEN dropdown's panel moves the cursor.
    if let Some(panel) = geometry.open_panel {
        if rect_contains(panel, column, row) {
            match kind {
                MouseEventKind::ScrollUp => {
                    view.emit(Action::MenuDropdownCursor(-1));
                    return Some(EventResult::consumed());
                }
                MouseEventKind::ScrollDown => {
                    view.emit(Action::MenuDropdownCursor(1));
                    return Some(EventResult::consumed());
                }
                _ => {}
            }
        }
    }
    // A left click that only closed the dropdown (outside, non-panel):
    // fall through to the normal header/content selection.
    if close_outside {
        return None;
    }
    None
}
