//! MUX-010 — the mux layout tree: nested columns and rows described by one
//! bracket layout string.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! A [`MuxLayout`] is a non-empty list of [`MuxNode`]s laid out as
//! COLUMNS. A [`MuxNode::Group`] splits its slot the other way, and the
//! direction alternates at every nesting level (top: columns; `[..]`:
//! rows; `[[..]]`: columns, ...). Every node carries a positive weight
//! (1 when the string has no `:N` suffix).
//!
//! Child modules:
//!   - `parse.rs`    — the layout grammar + exact column-carrying errors
//!   - `geometry.rs` — pane rects + dividers for an area
//!   - `fit.rs`      — the minimum pane size check
//!   - `spatial.rs`  — Shift+arrow neighbour search
//!   - `drag.rs`     — divider-drag re-weighting

pub mod drag;
pub mod fit;
pub mod geometry;
pub mod parse;
pub mod spatial;

use std::fmt;

use super::MuxPaneKind;

pub use fit::{check_fit, FitError, MIN_PANE_HEIGHT, MIN_PANE_WIDTH};
pub use geometry::{compute, Divider, Geometry};
pub use parse::MuxLayoutError;
pub use spatial::{neighbour, Direction};

/// One item of a layout list: a pane or a nested group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuxNode {
    Pane { kind: MuxPaneKind, weight: u32 },
    Group { children: Vec<MuxNode>, weight: u32 },
}

impl MuxNode {
    /// A weight-1 pane.
    pub fn pane(kind: MuxPaneKind) -> Self {
        Self::Pane { kind, weight: 1 }
    }

    pub fn weight(&self) -> u32 {
        match self {
            Self::Pane { weight, .. } | Self::Group { weight, .. } => *weight,
        }
    }

    pub(crate) fn set_weight(&mut self, value: u32) {
        match self {
            Self::Pane { weight, .. } | Self::Group { weight, .. } => *weight = value,
        }
    }

    /// Number of panes in this node (1 for a pane).
    pub fn leaf_count(&self) -> usize {
        match self {
            Self::Pane { .. } => 1,
            Self::Group { children, .. } => children.iter().map(Self::leaf_count).sum(),
        }
    }

    fn collect_leaves(&self, out: &mut Vec<MuxPaneKind>) {
        match self {
            Self::Pane { kind, .. } => out.push(*kind),
            Self::Group { children, .. } => {
                for child in children {
                    child.collect_leaves(out);
                }
            }
        }
    }
}

/// A whole mux layout: the top-level columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuxLayout {
    items: Vec<MuxNode>,
}

impl MuxLayout {
    /// Parse a layout string (see [`parse::parse_layout`]).
    pub fn parse(text: &str) -> Result<Self, MuxLayoutError> {
        parse::parse_layout(text)
    }

    /// Build a layout from parsed items. `None` for an empty list (a
    /// layout always has at least one pane).
    pub fn from_items(items: Vec<MuxNode>) -> Option<Self> {
        (!items.is_empty()).then_some(Self { items })
    }

    /// A layout from its first item and the rest (never empty).
    pub fn with_first(first: MuxNode, rest: Vec<MuxNode>) -> Self {
        let mut items = vec![first];
        items.extend(rest);
        Self { items }
    }

    /// One equal-weight column per pane, e.g. `board, agent`.
    pub fn columns(panes: &[MuxPaneKind]) -> Option<Self> {
        Self::from_items(panes.iter().copied().map(MuxNode::pane).collect())
    }

    /// The top-level items (columns).
    pub fn items(&self) -> &[MuxNode] {
        &self.items
    }

    /// The pane kinds in reading order (depth first, left to right).
    pub fn leaves(&self) -> Vec<MuxPaneKind> {
        let mut out = Vec::new();
        for item in &self.items {
            item.collect_leaves(&mut out);
        }
        out
    }

    /// The child list of the group at `path` (empty path = top level).
    pub(crate) fn group_children_mut(&mut self, path: &[usize]) -> Option<&mut Vec<MuxNode>> {
        let mut list = &mut self.items;
        for index in path {
            match list.get_mut(*index)? {
                MuxNode::Group { children, .. } => list = children,
                MuxNode::Pane { .. } => return None,
            }
        }
        Some(list)
    }
}

/// The layout-string name of a pane kind.
pub fn pane_name(kind: MuxPaneKind) -> &'static str {
    match kind {
        MuxPaneKind::Board => "board",
        MuxPaneKind::Agent => "agent",
        MuxPaneKind::ChangedFiles => "files",
        MuxPaneKind::Checkpoints => "checkpoints",
    }
}

fn write_list(f: &mut fmt::Formatter<'_>, items: &[MuxNode]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        match item {
            MuxNode::Pane { kind, .. } => f.write_str(pane_name(*kind))?,
            MuxNode::Group { children, .. } => {
                f.write_str("[")?;
                write_list(f, children)?;
                f.write_str("]")?;
            }
        }
        if item.weight() != 1 {
            write!(f, ":{}", item.weight())?;
        }
    }
    Ok(())
}

/// The canonical layout string: `, ` between items, `[..]` groups, `:N`
/// only for weights other than 1.
impl fmt::Display for MuxLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_list(f, &self.items)
    }
}

impl serde::Serialize for MuxLayout {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MuxLayout {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn canonical_string_round_trips() {
        for text in [
            "board, agent",
            "agent, [board, board], board, files",
            "agent:2, [board, board]:3, files",
            "[agent, [board, files]:2]",
            "checkpoints",
        ] {
            let layout = MuxLayout::parse(text).unwrap();
            assert_eq!(layout.to_string(), text);
        }
    }

    #[test]
    fn whitespace_and_weight_one_normalise() {
        let layout = MuxLayout::parse("  agent:1 ,[ board,board ] ").unwrap();
        assert_eq!(layout.to_string(), "agent, [board, board]");
    }

    #[test]
    fn leaves_are_in_reading_order() {
        let layout = MuxLayout::parse("agent, [board, [files, checkpoints]], board").unwrap();
        assert_eq!(
            layout.leaves(),
            vec![
                MuxPaneKind::Agent,
                MuxPaneKind::Board,
                MuxPaneKind::ChangedFiles,
                MuxPaneKind::Checkpoints,
                MuxPaneKind::Board,
            ]
        );
    }

    #[test]
    fn serde_goes_through_the_canonical_string() {
        let layout = MuxLayout::parse("agent:2, [board, files]").unwrap();
        let value = serde_json::to_value(&layout).unwrap();
        assert_eq!(value, serde_json::json!("agent:2, [board, files]"));
        let back: MuxLayout = serde_json::from_value(value).unwrap();
        assert_eq!(back, layout);
        let bad = serde_json::from_value::<MuxLayout>(serde_json::json!("agent, ["));
        assert!(bad.is_err());
    }
}
