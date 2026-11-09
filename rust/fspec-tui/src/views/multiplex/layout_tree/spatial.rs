//! MUX-010 — spatial focus movement across nested panes (Shift+Up/Down).
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! From the focused pane, the neighbour above/below is the nearest pane
//! wholly on that side whose span overlaps the focused pane's columns;
//! ties go to the largest overlap, then the left-most pane, then reading
//! order. MUX-012: Shift+Left/Right follow the layout order instead
//! (`focus_moves.rs`).

use ratatui::layout::Rect;

/// A vertical Shift+arrow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

fn overlap(a_start: u16, a_len: u16, b_start: u16, b_len: u16) -> u16 {
    let end = (a_start + a_len).min(b_start + b_len);
    end.saturating_sub(a_start.max(b_start))
}

/// The pane index to focus from `from` towards `direction`, or `None` at
/// an edge.
pub fn neighbour(panes: &[Rect], from: usize, direction: Direction) -> Option<usize> {
    let current = *panes.get(from)?;
    let mut best: Option<(u16, u16, u16, usize)> = None;
    for (index, rect) in panes.iter().enumerate() {
        if index == from {
            continue;
        }
        let (distance, shared, cross) = match direction {
            Direction::Down if rect.y >= current.bottom() => (
                rect.y - current.bottom(),
                overlap(current.x, current.width, rect.x, rect.width),
                rect.x,
            ),
            Direction::Up if rect.bottom() <= current.y => (
                current.y - rect.bottom(),
                overlap(current.x, current.width, rect.x, rect.width),
                rect.x,
            ),
            _ => continue,
        };
        if shared == 0 {
            continue;
        }
        // Smaller distance, larger overlap, smaller cross position win.
        let key = (distance, u16::MAX - shared, cross, index);
        if best.is_none_or(|b| key < b) {
            best = Some(key);
        }
    }
    best.map(|(_, _, _, index)| index)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::layout_tree::{compute, MuxLayout};

    #[test]
    fn moves_up_and_down_across_nested_panes() {
        let layout = MuxLayout::parse("agent, [board, [files:2, board]]").unwrap();
        let panes = compute(Rect::new(0, 1, 120, 38), &layout, &[]).panes;
        assert_eq!(neighbour(&panes, 1, Direction::Down), Some(2));
        // files overlaps the top board most.
        assert_eq!(neighbour(&panes, 3, Direction::Up), Some(1));
        assert_eq!(neighbour(&panes, 2, Direction::Up), Some(1));
        assert_eq!(neighbour(&panes, 1, Direction::Up), None);
        assert_eq!(neighbour(&panes, 0, Direction::Down), None);
    }
}
