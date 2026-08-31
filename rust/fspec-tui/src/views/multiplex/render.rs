//! MUX-001 — mux render: pane dispatch + divider + mux footer paint.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Splits the terminal area into a body (all but the bottom row) and a
//! 1-row mux footer. Panes are rendered into their sub-`Rect`s (views
//! use absolute coordinates, so no translation is needed). The divider
//! is painted on top; the focused pane's divider accent is highlighted.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::store::{AgentViewStore, BoardStore};
use crate::views::{AgentView, BoardView, ChangedFilesView, CheckpointsView};

use super::layout::{calculate_pane_rects_with_override, divider_rects};
use super::{MultiplexLayout, MuxOrientation};

/// MUX-005: dark purple background of the mux footer bar (white fg text).
/// Mirrors the named-bg pattern in views/agent/footer.rs (FOOTER_BG).
pub(crate) const MUX_FOOTER_BG: Color = Color::Rgb(74, 44, 112);

/// The four pane-hosting views, bundled so the render entry point stays
/// under the clippy argument-count ceiling.
pub struct MuxRenderViews<'a> {
    pub board: &'a BoardView,
    pub agent: &'a mut AgentView,
    pub changed_files: &'a mut ChangedFilesView,
    pub checkpoints: &'a mut CheckpointsView,
}

/// Paint the mux grid into `area`.
pub fn render_with_stores(
    layout: &mut MultiplexLayout,
    area: Rect,
    buf: &mut Buffer,
    board_store: &BoardStore,
    agent_store: &mut AgentViewStore,
    views: &mut MuxRenderViews<'_>,
) {
    if !layout.config.enabled || area.height < 3 || area.width < 2 {
        return;
    }
    // MUX-002: keep the agent window in sync with the live open-session
    // list — unfilled agent slots are dropped from the rendered pane
    // list (no blank panes) and the window re-clamps after a close.
    let session_ids: Vec<codelet_rpc_types::SessionId> = agent_store
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    layout.sync_window(&session_ids);

    // BUG-163: the agent window in pane order — agent slot `i` renders the
    // session at `window_start + i`. Previously the window math only drove
    // the pane LIST; every agent pane still painted the store's current
    // session, duplicating the focused session into the other panes.
    let window_sessions = layout.window_session_ids();

    let body = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height - 1, // reserve the footer row
    };
    layout.body_area = Some(body);

    let rects = calculate_pane_rects_with_override(
        body,
        layout.config.orientation,
        layout.effective_panes(),
        &layout.config.splits,
        layout.drag_index,
        layout.drag_width,
    );
    layout.pane_rects = rects.clone();

    // BUG-166: divider rects — one per inter-pane gap.
    layout.divider_rects = divider_rects(layout.config.orientation, &rects, body);

    let focus = layout.focus();
    for (i, rect) in rects.iter().enumerate() {
        let kind = layout.effective_panes().get(i).copied().unwrap_or_default();
        match kind {
            super::MuxPaneKind::Board => views.board.render_with_store(*rect, buf, board_store),
            super::MuxPaneKind::Agent => {
                // BUG-163: each agent pane paints the session at its
                // window slot (Nth agent slot in the rendered list →
                // `window_sessions[n]`); the focused agent pane hosts
                // the live composer, the others paint a read-only ghost
                // draft of their own session.
                let agent_slot = layout
                    .effective_panes()
                    .iter()
                    .take(i)
                    .filter(|k| **k == super::MuxPaneKind::Agent)
                    .count();
                let pane = crate::views::agent::pane_render::PaneSession {
                    session: window_sessions.get(agent_slot).cloned(),
                    is_focused: i == focus,
                };
                views
                    .agent
                    .render_session_pane(*rect, buf, agent_store, pane)
            }
            super::MuxPaneKind::ChangedFiles => views.changed_files.render(*rect, buf),
            super::MuxPaneKind::Checkpoints => views.checkpoints.render(*rect, buf),
        }
    }

    // MUX-006/MUX-008: tint the flash over the newly-focused pane's
    // FULL rect — AFTER the pane content (the flash is a
    // background-only overlay, R1/R5) and BEFORE the dividers/footer
    // (it never touches divider columns or the footer row). The clock
    // advances AFTER the paint so frame 1 renders the bottom edge
    // (clock 0) and the final frame the top edge (clock 336).
    // MUX-007/MUX-008: the paint gate is the SETTLED accent (focused
    // pane owns the row) — during the 350ms window the row sweeps
    // bottom-to-top, and once the window elapses the same pattern fn
    // settles at the top row (a 1-row bar across the full width) and
    // keeps painting there on every frame of the focused pane
    // (repaint content, not an animation — the tick gate stays
    // closed, R4).
    paint_focus_flash(layout, &rects, buf);
    layout.advance_flash_clock();

    paint_dividers(layout, buf);
    paint_footer(layout, area, buf);
}

/// MUX-006/MUX-007/MUX-008: paint the focus-flash cells (dark purple
/// background only — `set_style` without `set_symbol`, so pane glyphs
/// stay readable) over the armed pane's rect. The gate is the SETTLED
/// accent (`has_settled_flash`): the row paints during the 350ms
/// bottom-to-top scan and, once the window elapses, keeps painting the
/// parked top-row bar on every frame of the focused pane (R1). No-op
/// with no accent armed (mux not entered / disabled — R7).
fn paint_focus_flash(layout: &MultiplexLayout, rects: &[Rect], buf: &mut Buffer) {
    if !layout.has_settled_flash() {
        return;
    }
    let Some(pane) = layout.flash_pane() else {
        return;
    };
    let Some(rect) = rects.get(pane) else {
        return;
    };
    let cells = super::flash::flash_cells(*rect, layout.flash_clock_ms());
    if cells.is_empty() {
        return;
    }
    let style = Style::default().bg(MUX_FOOTER_BG);
    for (x, y) in cells {
        if buf.area.contains(ratatui::layout::Position { x, y }) {
            buf[(x, y)].set_style(style);
        }
    }
}

/// BUG-166: paint every inter-pane divider. The divider currently being
/// dragged is highlighted (cyan); the rest stay dimmed.
fn paint_dividers(layout: &MultiplexLayout, buf: &mut Buffer) {
    let glyph = match layout.config.orientation {
        MuxOrientation::Horizontal => "│",
        MuxOrientation::Vertical => "─",
    };
    for (i, divider) in layout.divider_rects.iter().enumerate() {
        let style = if layout.is_dragging && layout.drag_index == Some(i) {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        for y in divider.y..divider.y + divider.height {
            for x in divider.x..divider.x + divider.width {
                if buf.area.contains(ratatui::layout::Position { x, y }) {
                    buf[(x, y)].set_symbol(glyph).set_style(style);
                }
            }
        }
    }
}

fn paint_footer(layout: &MultiplexLayout, area: Rect, buf: &mut Buffer) {
    let row = area.y + area.height - 1;
    let mut label = format!(
        "MUX {} panes [{}]",
        layout.effective_panes().len(),
        layout
            .effective_panes()
            .iter()
            .map(|k| match k {
                super::MuxPaneKind::Board => "Board",
                super::MuxPaneKind::Agent => "Agent",
                super::MuxPaneKind::ChangedFiles => "Files",
                super::MuxPaneKind::Checkpoints => "Checkpoints",
            })
            .collect::<Vec<_>>()
            .join("|")
    );
    label.push_str(&format!("  ●pane {}", layout.focus()));
    label.push_str("  /mux config · Shift+←/→ focus · drag divider");
    // MUX-005: white text on a dark purple bar spanning the FULL row
    // width (label cells + the tail cells past the label).
    let style = Style::default().fg(Color::White).bg(MUX_FOOTER_BG);
    for (i, ch) in label.chars().enumerate() {
        let x = area.x + i as u16;
        if x >= area.x + area.width {
            break;
        }
        buf[(x, row)].set_symbol(&ch.to_string()).set_style(style);
    }
    let label_end = (area.x + label.chars().count() as u16).min(area.x + area.width);
    for x in label_end..area.x + area.width {
        buf[(x, row)].set_symbol(" ").set_style(style);
    }
}
