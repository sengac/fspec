//! RPC-412 — freeform HITL cursor-anchor geometry for the AgentView.
//!
//! Feature: spec/features/inline-hitl-freeform-cursor-position.feature
//!
//! Extracted from `views/agent.rs` to keep the orchestrator under its
//! 300-LoC source-shape ceiling. In freeform HITL mode the shared "> "
//! composer input is painted `last_hitl_input_offset` rows below the
//! input-area top (below the wrapped header + optional empty-submit
//! hint). This module shifts the cursor anchor down by that offset so
//! the hardware block cursor sits on the input line, not the header —
//! the region height shrinks by the same offset so the viewport clamp
//! lets the cursor reach the true input row instead of being pulled
//! back up onto the header. The X/column math is unchanged.

use ratatui::layout::Rect;

/// The input-area rect the hardware cursor should be positioned inside:
/// the freeform header offset shifts `y` down and shrinks the height so
/// the clamp reaches the painted "> " input row. `None` offset (normal
/// composer, options-mode HITL, pause prompt) returns `area` unchanged.
pub(crate) fn anchored_input_area(area: Rect, offset: Option<u16>) -> Rect {
    match offset {
        Some(offset) => Rect {
            x: area.x,
            y: area.y.saturating_add(offset),
            width: area.width,
            height: area.height.saturating_sub(offset.min(area.height)),
        },
        None => area,
    }
}
