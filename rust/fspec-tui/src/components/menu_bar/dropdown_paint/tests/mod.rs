#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

use crate::components::menu_bar::dropdown::dropdown_rect;

use ratatui::buffer::Buffer;

/// The 'Kanban' item x in a standard 80x24 bar (1-cell R1 pad).
const KANBAN_X: u16 = 1;

fn bar_area(w: u16, h: u16) -> Rect {
    Rect::new(0, 0, w, h)
}

mod part_1;
