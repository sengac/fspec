//! MUX-010 — divider drags re-weight the two neighbours of a group.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! The pair's joint weight is re-split by the percent of their joint size
//! the item before the divider now takes (1..=99), every other weight of
//! the group is scaled by 100 to match, and the group is then reduced by
//! its greatest common divisor — a 1:1 pair dragged to 40/60 becomes 2:3.

use super::{Divider, MuxLayout};

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Re-weight the pair around `divider` in `layout` so the item before it
/// takes `before_cells` of their joint `divider.pair` cells. Returns
/// `false` when the divider no longer addresses the layout.
pub fn reweight(layout: &mut MuxLayout, divider: &Divider, before_cells: u16) -> bool {
    let Some(children) = layout.group_children_mut(&divider.group) else {
        return false;
    };
    if divider.before >= children.len() || divider.after >= children.len() {
        return false;
    }
    let pair = u32::from(divider.pair.max(1));
    let cells = u32::from(before_cells).min(pair);
    let percent = ((cells * 100 + pair / 2) / pair).clamp(1, 99);
    let joint = children[divider.before].weight() + children[divider.after].weight();
    let mut weights: Vec<u32> = children.iter().map(|c| c.weight() * 100).collect();
    weights[divider.before] = joint * percent;
    weights[divider.after] = joint * (100 - percent);
    let common = weights.iter().copied().fold(0, gcd).max(1);
    for (child, weight) in children.iter_mut().zip(weights) {
        child.set_weight(weight / common);
    }
    true
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::multiplex::layout_tree::compute;
    use ratatui::layout::Rect;

    fn drag(text: &str, divider: usize, before_cells: u16, width: u16) -> String {
        let mut layout = MuxLayout::parse(text).unwrap();
        let g = compute(Rect::new(0, 0, width, 20), &layout, &[]);
        assert!(reweight(&mut layout, &g.dividers[divider], before_cells));
        layout.to_string()
    }

    #[test]
    fn a_dragged_pair_gets_reduced_weights() {
        assert_eq!(drag("agent, board", 0, 48, 120), "agent:2, board:3");
        assert_eq!(drag("board, agent", 0, 40, 120), "board:17, agent:33");
        assert_eq!(
            drag("board, agent, files", 1, 40, 120),
            "board:50, agent:51, files:49"
        );
        // dividers[0] is the row divider inside the group (9 of 19 rows).
        assert_eq!(
            drag("[board, files], agent", 0, 9, 61),
            "[board:47, files:53], agent"
        );
    }
}
