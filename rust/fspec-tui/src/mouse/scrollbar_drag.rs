//! Scrollbar click-and-drag navigation state machine (TUI-101).
//!
//! Feature: spec/features/scrollbar-click-and-drag-navigation-core-module.feature
//!
//! Pure state machine — no view imports, no Action enum knowledge.
//! Consumer handles hit-testing and state application.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Geometry passed by the consumer when a Down event occurs.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarGeometry {
    /// Height of the scrollbar area in rows.
    pub area_height: usize,
    /// Total number of items in the scrollable list.
    pub total_items: usize,
    /// Number of visible items (viewport height).
    pub visible_items: usize,
    /// Current scroll offset (index of first visible item).
    pub current_offset: usize,
}

/// Internal scrollbar drag state.
enum State {
    Idle,
    /// Left button pressed; geometry captured at press time.
    Pressed { geom: ScrollbarGeometry },
    /// Drag in progress; geometry still from press time.
    Dragging { geom: ScrollbarGeometry },
}

/// Scrollbar click-and-drag state machine.
pub struct ScrollbarDrag {
    state: State,
}

impl Default for ScrollbarDrag {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollbarDrag {
    /// Construct a fresh handler in the idle state.
    pub fn new() -> Self {
        Self { state: State::Idle }
    }

    /// Feed a mouse event with current scrollbar geometry.
    ///
    /// Returns `Some(offset)` when the consumer should jump to that
    /// scroll offset, `None` when no action is needed.
    pub fn on_mouse(&mut self, ev: MouseEvent, geom: ScrollbarGeometry) -> Option<usize> {
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.state = State::Pressed { geom };
                None
            }
            MouseEventKind::Drag(MouseButton::Left) => match &self.state {
                State::Pressed { geom: stored } => {
                    let offset = Self::compute_offset(ev.row, stored);
                    self.state = State::Dragging { geom: *stored };
                    Some(offset)
                }
                State::Dragging { geom: stored } => {
                    Some(Self::compute_offset(ev.row, stored))
                }
                State::Idle => None,
            },
            MouseEventKind::Up(MouseButton::Left) => {
                let prev = std::mem::replace(&mut self.state, State::Idle);
                match prev {
                    State::Pressed { geom: stored } => {
                        Some(Self::handle_quick_click(ev.row, &stored))
                    }
                    State::Dragging { .. } => None,
                    State::Idle => None,
                }
            }
            _ => None,
        }
    }

    /// Reset to idle (called when content changes).
    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    /// True when a drag is in progress.
    pub fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging { .. })
    }

    /// True when the state machine is not idle (either Pressed or Dragging).
    /// Used by consumers to decide whether Drag/Up events should be routed
    /// back to the scrollbar handler.
    pub fn is_active(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Invert the proportional formula: offset = (row * total) / area_height.
    fn compute_offset(row: u16, geom: &ScrollbarGeometry) -> usize {
        if geom.total_items == 0 || geom.area_height == 0 {
            return 0;
        }
        if geom.total_items <= geom.visible_items {
            return 0;
        }
        let offset = (row as usize * geom.total_items) / geom.area_height;
        let max_offset = geom.total_items.saturating_sub(geom.visible_items);
        offset.min(max_offset)
    }

    /// Handle a quick click (Up from Pressed without an intervening Drag).
    fn handle_quick_click(row: u16, geom: &ScrollbarGeometry) -> usize {
        if geom.total_items <= geom.visible_items {
            return 0;
        }
        let thumb_start = (geom.current_offset * geom.area_height) / geom.total_items;
        let thumb_h = ((geom.visible_items * geom.area_height) / geom.total_items).max(1);
        let thumb_end = thumb_start + thumb_h;

        if (row as usize) >= thumb_start && (row as usize) < thumb_end {
            // Click on thumb: scroll one viewport height toward click direction
            let thumb_mid = thumb_start + thumb_h / 2;
            if (row as usize) < thumb_mid {
                // Upper half — scroll down
                let max_offset = geom.total_items.saturating_sub(geom.visible_items);
                (geom.current_offset + geom.visible_items).min(max_offset)
            } else {
                // Lower half — scroll up
                geom.current_offset.saturating_sub(geom.visible_items)
            }
        } else {
            // Click on track: jump to click position
            Self::compute_offset(row, geom)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/scrollbar-click-and-drag-navigation-core-module.feature
    use super::*;
    use crossterm::event::KeyModifiers;

    fn ev(kind: MouseEventKind, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: 0,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    fn geom(area_height: usize, total: usize, visible: usize, offset: usize) -> ScrollbarGeometry {
        ScrollbarGeometry {
            area_height,
            total_items: total,
            visible_items: visible,
            current_offset: offset,
        }
    }

    #[test]
    fn click_on_track_above_thumb_jumps_to_that_position() {
        // @step Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
        let g = geom(20, 100, 10, 0);
        // @step And the current scroll offset is 0 (thumb occupies rows 0-1)
        let mut drag = ScrollbarDrag::new();
        // @step When I click the left mouse button at row 5 on the scrollbar track
        let down = drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 5), g);
        assert_eq!(down, None);
        let up = drag.on_mouse(ev(MouseEventKind::Up(MouseButton::Left), 5), g);
        // @step Then the ScrollbarDrag should return a scroll offset of 25
        assert_eq!(up, Some(25));
        // @step And the state should return to idle after the click
        assert!(!drag.is_dragging());
    }

    #[test]
    fn click_on_track_below_thumb_jumps_to_that_position() {
        // @step Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
        let g = geom(20, 100, 10, 0);
        // @step And the current scroll offset is 0 (thumb occupies rows 0-1)
        let mut drag = ScrollbarDrag::new();
        // @step When I click the left mouse button at row 15 on the scrollbar track
        let down = drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 15), g);
        assert_eq!(down, None);
        let up = drag.on_mouse(ev(MouseEventKind::Up(MouseButton::Left), 15), g);
        // @step Then the ScrollbarDrag should return a scroll offset of 75
        assert_eq!(up, Some(75));
        // @step And the state should return to idle after the click
        assert!(!drag.is_dragging());
    }

    #[test]
    fn click_and_drag_thumb_continuously_updates_scroll_offset() {
        // @step Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
        let g = geom(20, 100, 10, 0);
        // @step And the current scroll offset is 0
        let mut drag = ScrollbarDrag::new();
        // @step When I press the left mouse button on the thumb at row 0
        let down = drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 0), g);
        assert_eq!(down, None);
        // @step And I drag the mouse down to row 10
        let drag_result = drag.on_mouse(ev(MouseEventKind::Drag(MouseButton::Left), 10), g);
        // @step Then the ScrollbarDrag should return a scroll offset of 50 during the drag
        assert_eq!(drag_result, Some(50));
        // @step And releasing the mouse button should return the state to idle
        let up = drag.on_mouse(ev(MouseEventKind::Up(MouseButton::Left), 10), g);
        assert_eq!(up, None);
        assert!(!drag.is_dragging());
    }

    #[test]
    fn quick_click_on_thumb_without_drag_scrolls_one_viewport_height() {
        // @step Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
        let g = geom(20, 100, 10, 0);
        // @step And the current scroll offset is 0
        let mut drag = ScrollbarDrag::new();
        // @step When I quickly click and release the left mouse button on the thumb at row 0 without dragging
        let down = drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 0), g);
        assert_eq!(down, None);
        let up = drag.on_mouse(ev(MouseEventKind::Up(MouseButton::Left), 0), g);
        // @step Then the ScrollbarDrag should return a scroll offset of 10 (one viewport height)
        assert_eq!(up, Some(10));
    }

    #[test]
    fn drag_continues_when_cursor_strays_outside_scrollbar_area() {
        // @step Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
        let g = geom(20, 100, 10, 0);
        // @step And I have pressed the left mouse button on the thumb
        let mut drag = ScrollbarDrag::new();
        drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 0), g);
        // @step When I drag the mouse to row 15 even if it moves outside the scrollbar rect
        let result = drag.on_mouse(ev(MouseEventKind::Drag(MouseButton::Left), 15), g);
        // @step Then the ScrollbarDrag should still compute and return the scroll offset for row 15
        assert_eq!(result, Some(75));
    }

    #[test]
    fn non_left_button_events_are_ignored() {
        // @step Given a ScrollbarDrag in idle state
        let mut drag = ScrollbarDrag::new();
        let g = geom(20, 100, 10, 0);
        // @step When I scroll the mouse wheel up
        let result = drag.on_mouse(ev(MouseEventKind::ScrollUp, 5), g);
        // @step Then the ScrollbarDrag should return None (no action)
        assert_eq!(result, None);
        // @step And the state should remain idle
        assert!(!drag.is_dragging());
    }

    #[test]
    fn reset_clears_dragging_state() {
        // @step Given a ScrollbarDrag in the middle of a drag operation
        let g = geom(20, 100, 10, 0);
        let mut drag = ScrollbarDrag::new();
        drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 0), g);
        drag.on_mouse(ev(MouseEventKind::Drag(MouseButton::Left), 10), g);
        assert!(drag.is_dragging());
        // @step When reset is called
        drag.reset();
        // @step Then the state should return to idle
        assert!(!drag.is_dragging());
        // @step And no scroll offset should be returned
        let result = drag.on_mouse(ev(MouseEventKind::Drag(MouseButton::Left), 5), g);
        assert_eq!(result, None);
    }

    #[test]
    fn no_scrollbar_needed_when_content_fits_in_viewport() {
        // @step Given a scrollbar with 5 total items and 10 visible rows
        let g = geom(20, 5, 10, 0);
        let mut drag = ScrollbarDrag::new();
        // @step When I click at any row on the scrollbar
        let down = drag.on_mouse(ev(MouseEventKind::Down(MouseButton::Left), 5), g);
        assert_eq!(down, None);
        let up = drag.on_mouse(ev(MouseEventKind::Up(MouseButton::Left), 5), g);
        // @step Then the ScrollbarDrag should return a scroll offset of 0
        assert_eq!(up, Some(0));
        // @step And the state should return to idle
        assert!(!drag.is_dragging());
    }
}
