//! MENU-003 + MENU-007 — the agent view's 2-zone menu bar mouse arms.
//!
//! Feature: spec/features/agent-view-surface-2-zone-bar-row-under-session-header.feature
//! Card: MENU-003 (R5, R7).
//!
//! Hit-test order (R7: the bar row hit-test runs BEFORE the
//! input/scrollback tests — the caller invokes this at the top of the
//! mouse branch):
//!
//! 1. **A left click on the bar row** — the 'Board View' item
//!    ACTIVATES it (returns to the Board view, MENU-007 — no
//!    dropdowns in the agent view); a chip activates (R5, same as
//!    Enter on the chip).
//! 2. **Wheel over the bar row** — `ScrollLeft/ScrollRight` walk the
//!    ring exactly like the keys (R2/R7 parity) but ONLY while a bar
//!    item is focused (unfocused bar → ignored, R7).
//!
//! The render path refreshes the per-frame geometry (item rects, cell
//! rects, bar row) in `menu_state` (focused pane only — the BUG-163
//! rule); this module reads them back. Inert while the last paint had
//! no bar row (`menu_state.bar_row() == None`).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::components::menu_bar::DisplayCell;
use crate::components::EventResult;
use crate::mouse::rect_contains;

use super::AgentView;

impl AgentView {
    /// MENU-003: route a mouse event against the cached bar geometry.
    /// `Some(result)` when a menu arm claimed the event; `None` when
    /// nothing matched (the caller's normal input/scrollback arms run
    /// on the same event).
    pub(crate) fn handle_menu_mouse(&mut self, event: MouseEvent) -> Option<EventResult> {
        let bar_row = self.menu_state.bar_row()?;
        let MouseEvent {
            kind, column, row, ..
        } = event;

        if row == bar_row {
            match kind {
                // R7: wheel L/R over the bar row walks the ring — but
                // ONLY while a bar item is focused (the scenario: wheel
                // over the bar with no bar focus is ignored).
                MouseEventKind::ScrollLeft if self.menu_state.focus().is_some() => {
                    self.walk_menu_ring(-1);
                    return Some(EventResult::consumed());
                }
                MouseEventKind::ScrollRight if self.menu_state.focus().is_some() => {
                    self.walk_menu_ring(1);
                    return Some(EventResult::consumed());
                }
                // R7: the 'Board View' item activates (returns to the
                // Board view, MENU-007); a chip activates.
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(items) = self.menu_state.item_rects() {
                        for rect in items.iter() {
                            if rect_contains(*rect, column, row) {
                                self.activate_board_view_item();
                                return Some(EventResult::consumed());
                            }
                        }
                    }
                    if let Some(cells) = self.menu_state.cells() {
                        for (cell, rect) in cells.iter() {
                            if rect_contains(*rect, column, row) {
                                if let DisplayCell::Chip { orig } = cell {
                                    self.activate_menu_chip(*orig);
                                    return Some(EventResult::consumed());
                                }
                            }
                        }
                    }
                    return Some(EventResult::ignored());
                }
                _ => {}
            }
        }

        None
    }
}
