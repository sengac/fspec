//! MUX-010 — layout geometry: pane rects and dividers for an area.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! Each list is split along its direction (depth 0: columns, then
//! alternating) with one 1-cell divider between visible neighbours. A
//! visible item gets `floor(available × weight / total)` cells and the
//! last visible item takes the remainder (the MUX-003 equal division when
//! every weight is 1). Hidden panes (unfilled agent slots, Esc-closed
//! panes) take no space; a group with no visible pane disappears.

use ratatui::layout::Rect;

use super::{MuxLayout, MuxNode};
use crate::views::multiplex::MuxPaneKind;

/// One divider between two visible neighbours of a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divider {
    pub rect: Rect,
    /// True when the divider separates columns (a vertical line).
    pub between_columns: bool,
    /// Path of the group in the layout (empty = top level).
    pub group: Vec<usize>,
    /// Child indexes (in the stored group) of the item before / after.
    pub before: usize,
    pub after: usize,
    /// Start cell of the item before (x or y) and the two items' joint
    /// size in cells (dividers excluded).
    pub start: u16,
    pub pair: u16,
}

/// The computed geometry: visible panes in reading order + dividers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geometry {
    pub panes: Vec<Rect>,
    pub kinds: Vec<MuxPaneKind>,
    pub dividers: Vec<Divider>,
}

/// Compute the geometry of `layout` in `area`. `visible[i]` says whether
/// leaf `i` (reading order) is shown; a short mask shows the rest.
pub fn compute(area: Rect, layout: &MuxLayout, visible: &[bool]) -> Geometry {
    let mut geometry = Geometry::default();
    if area.width == 0 || area.height == 0 {
        return geometry;
    }
    let mut ctx = Ctx {
        visible,
        out: &mut geometry,
    };
    ctx.place(area, layout.items(), 0, &mut Vec::new(), 0);
    geometry
}

struct Ctx<'a> {
    visible: &'a [bool],
    out: &'a mut Geometry,
}

impl Ctx<'_> {
    fn leaf_visible(&self, leaf: usize) -> bool {
        self.visible.get(leaf).copied().unwrap_or(true)
    }

    fn node_visible(&self, node: &MuxNode, first_leaf: usize) -> bool {
        (first_leaf..first_leaf + node.leaf_count()).any(|leaf| self.leaf_visible(leaf))
    }

    fn place(
        &mut self,
        area: Rect,
        items: &[MuxNode],
        depth: usize,
        path: &mut Vec<usize>,
        first_leaf: usize,
    ) {
        // (child index, first leaf, node) of every visible child.
        let mut shown = Vec::new();
        let mut leaf = first_leaf;
        for (index, node) in items.iter().enumerate() {
            if self.node_visible(node, leaf) {
                shown.push((index, leaf, node));
            }
            leaf += node.leaf_count();
        }
        if shown.is_empty() {
            return;
        }
        let columns = depth.is_multiple_of(2);
        let (origin, span) = if columns {
            (area.x, area.width)
        } else {
            (area.y, area.height)
        };
        let gaps = (shown.len() - 1) as u16;
        let weights: Vec<u32> = shown.iter().map(|(_, _, node)| node.weight()).collect();
        let sizes = split(span.saturating_sub(gaps), &weights);
        let mut cursor = origin;
        for (i, ((index, leaf, node), size)) in shown.iter().zip(&sizes).enumerate() {
            let rect = if columns {
                Rect::new(cursor, area.y, *size, area.height)
            } else {
                Rect::new(area.x, cursor, area.width, *size)
            };
            match node {
                MuxNode::Pane { kind, .. } => {
                    self.out.panes.push(rect);
                    self.out.kinds.push(*kind);
                }
                MuxNode::Group { children, .. } => {
                    path.push(*index);
                    self.place(rect, children, depth + 1, path, *leaf);
                    path.pop();
                }
            }
            if let Some((next, _, _)) = shown.get(i + 1) {
                let at = cursor + size;
                let divider = if columns {
                    Rect::new(at, area.y, 1, area.height)
                } else {
                    Rect::new(area.x, at, area.width, 1)
                };
                self.out.dividers.push(Divider {
                    rect: divider,
                    between_columns: columns,
                    group: path.clone(),
                    before: *index,
                    after: *next,
                    start: cursor,
                    pair: size + sizes[i + 1],
                });
            }
            cursor += size + 1;
        }
    }
}

/// Split `available` cells by `weights`: `floor(available × w / total)`
/// each, every item at least 1 cell when there is room, the last item
/// the remainder. With less room than items, the first items get 1 cell.
pub fn split(available: u16, weights: &[u32]) -> Vec<u16> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    if (available as usize) < n {
        return (0..n).map(|i| u16::from(i < available as usize)).collect();
    }
    let total: u64 = weights.iter().map(|w| u64::from(*w)).sum::<u64>().max(1);
    let mut sizes: Vec<u16> = weights[..n - 1]
        .iter()
        .map(|w| ((u64::from(available) * u64::from(*w) / total) as u16).max(1))
        .collect();
    let mut used: u16 = sizes.iter().sum();
    // Leave the last item at least one cell, shrinking the largest.
    while used > available - 1 {
        if let Some(largest) = sizes.iter_mut().max() {
            *largest -= 1;
        }
        used -= 1;
    }
    sizes.push(available - used);
    sizes
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn widths(text: &str, w: u16, h: u16) -> Geometry {
        compute(Rect::new(0, 0, w, h), &MuxLayout::parse(text).unwrap(), &[])
    }

    #[test]
    fn equal_weights_keep_the_mux003_division() {
        assert_eq!(split(119, &[1, 1]), vec![59, 60]);
        assert_eq!(split(77, &[1, 1, 1, 1]), vec![19, 19, 19, 20]);
        assert_eq!(split(118, &[2, 1, 1]), vec![59, 29, 30]);
        assert_eq!(split(2, &[1, 1, 1]), vec![1, 1, 0]);
        assert_eq!(split(3, &[100, 1, 1]), vec![1, 1, 1]);
    }

    #[test]
    fn nested_rows_and_dividers() {
        let g = widths("agent, [board, board], board, files", 80, 22);
        let xs: Vec<(u16, u16)> = g.panes.iter().map(|r| (r.x, r.width)).collect();
        assert_eq!(xs, vec![(0, 19), (20, 19), (20, 19), (40, 19), (60, 20)]);
        assert_eq!((g.panes[1].y, g.panes[1].height), (0, 10));
        assert_eq!((g.panes[2].y, g.panes[2].height), (11, 11));
        assert_eq!(g.dividers.len(), 4);
        let row_divider = g.dividers.iter().find(|d| !d.between_columns).unwrap();
        assert_eq!(row_divider.rect, Rect::new(20, 10, 19, 1));
        assert_eq!(row_divider.group, vec![1]);
    }

    #[test]
    fn hidden_leaves_take_no_space() {
        let layout = MuxLayout::parse("[agent, agent], board").unwrap();
        let g = compute(Rect::new(0, 0, 120, 38), &layout, &[true, false, true]);
        assert_eq!(
            g.panes,
            vec![Rect::new(0, 0, 59, 38), Rect::new(60, 0, 60, 38)]
        );
        assert_eq!(g.dividers.len(), 1);
    }
}
