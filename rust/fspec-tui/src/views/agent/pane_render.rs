//! BUG-163 — per-pane session targeting for `AgentView` renders.
//!
//! Feature: spec/features/mux-agent-panes-render-distinct-window-sessions.feature
//!
//! `AgentView` keeps ONE live `MultiLineInput` (the focused session's
//! composer). In mux mode each agent pane paints the session at its
//! window slot: the focused agent pane paints the live composer, every
//! other agent pane paints a ghost of its own session's persisted
//! `input_draft`. `PaneSession` carries the per-pane selection into
//! `AgentView::render_session_pane` without touching the live input
//! state of unfocused panes.

use ratatui::buffer::Buffer;
use ratatui::layout::{Direction, Layout, Rect};

use codelet_rpc_types::SessionStatus;

use crate::store::AgentViewStore;

use super::chrome_paint;
use super::AgentView;

/// Which session an agent pane paints + whether it hosts the live composer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneSession {
    /// `None` = the store's current session (single-view mode and the
    /// focused mux agent pane). `Some(sid)` = a specific window-session
    /// (a mux agent pane).
    pub session: Option<codelet_rpc_types::SessionId>,
    /// True iff this pane hosts the live composer (the focused agent
    /// pane). Unfocused panes render a read-only ghost draft and never
    /// mutate the live input / viewport state.
    pub is_focused: bool,
}

impl PaneSession {
    /// Single-view mode: paint the store's current session with the
    /// live composer.
    pub fn current_session() -> Self {
        Self {
            session: None,
            is_focused: true,
        }
    }
}

/// RPC-029: the RoleBanner row is present (height 1) iff the session has
/// a role; extracted so `render_session_pane` stays readable.
fn role_banner_height(store: &AgentViewStore, sid: Option<&codelet_rpc_types::SessionId>) -> u16 {
    sid.and_then(|s| store.role_for(s)).map(|_| 1).unwrap_or(0)
}

impl AgentView {
    /// BUG-163 — pane-targeted render: paints the session selected by
    /// `pane` (the store's current session for single-view mode, or the
    /// mux agent pane's window session) with either the live composer
    /// (focused pane) or a read-only ghost draft (unfocused pane). The
    /// view-level `last_*` caches (input area, scrollback geometry) are
    /// only updated by the FOCUSED pane so mouse hit-testing and the
    /// hardware cursor stay bound to the live composer.
    pub fn render_session_pane(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        store: &mut AgentViewStore,
        pane: PaneSession,
    ) {
        if let Some(v) = self.resume_view.as_mut() {
            v.render(area, buf);
            return;
        }
        if let Some(v) = self.search_view.as_mut() {
            v.render(area, buf);
            return;
        }
        // BUG-163: the pane's session — single-view mode keeps the store's
        // focused session; mux agent panes paint their window slot.
        let sid: Option<codelet_rpc_types::SessionId> =
            pane.session.or_else(|| store.current_session().cloned());
        // BUG-163: only the focused pane drives the live-composer height
        // (pause/HITL prompts + the shared MultiLineInput wrap); unfocused
        // panes paint a 1-row ghost draft instead.
        let input_height = if pane.is_focused {
            self.input_area_height(store, sid.as_ref(), area.width)
        } else {
            1
        };
        // RPC-029 layout: Header(1), RoleBanner(0|1), Scrollback flex Min(0), Footer Length(1), Input Length(input_height).
        // BUG-163: the constraint list itself is pinned on AgentView
        // (`pane_layout_constraints`) for the rpc013 source-shape test.
        let constraints = AgentView::pane_layout_constraints(
            role_banner_height(store, sid.as_ref()),
            input_height,
        );
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        let areas = chrome_paint::ChromeAreas {
            header: split[0],
            role: split[1],
            scrollback: split[2],
            footer: split[3],
            input: split[4],
        };
        // BUG-163: the view-level geometry caches back mouse hit-testing
        // (mouse_dispatch.rs) and the hardware cursor (cursor_position) —
        // both belong to the LIVE composer, so only the focused pane may
        // refresh them. Unfocused panes still paint the chrome from the
        // `areas` above (computed locally), just without caching.
        if pane.is_focused {
            self.last_render_area = Some(area);
            self.last_input_area = Some(areas.input);
            self.last_scrollback_viewport = areas.scrollback.height;
            self.last_scrollback_area = Some(areas.scrollback);
        }
        // BUG-163: tick the live-composer animation only for the focused
        // pane — spinner/transition state belongs to the live session.
        // Unfocused panes read the status directly (no animation tick).
        let (session_status, is_loading) = if pane.is_focused {
            self.tick_animation(store, sid.as_ref())
        } else {
            let status = sid
                .as_ref()
                .and_then(|s| store.session_status_for(s).copied());
            (status, matches!(status, Some(SessionStatus::Running)))
        };
        if pane.is_focused {
            self.last_is_compacting = matches!(session_status, Some(SessionStatus::Compacting));
        }
        let session_index = sid
            .as_ref()
            .map(|s| store.session_index_for(s))
            .unwrap_or_else(|| store.session_index());
        chrome_paint::paint_header_and_role(
            &areas,
            buf,
            store,
            sid.as_ref(),
            is_loading,
            self.turn_select_mode,
            session_index,
        );

        if let Some(sid) = sid.as_ref() {
            if let Some(ctx) = store.session_context_mut_for(sid) {
                ctx.scrollback.render_count_visited(areas.scrollback, buf);
                if pane.is_focused {
                    // TUI-102: cache total visual rows + scroll offset for scrollbar geometry.
                    self.last_scrollback_total_rows = ctx.scrollback.total_visual_rows();
                    self.last_scrollback_scroll_offset = ctx.scrollback.scroll_state().offset;
                    // TUI-102: reset drag state when scrollbar disappears.
                    let total = self.last_scrollback_total_rows;
                    let viewport = self.last_scrollback_viewport as usize;
                    if !(total > viewport && areas.scrollback.width >= 4) {
                        self.scrollback_scrollbar_drag.reset();
                    }
                }
            }
        }
        chrome_paint::paint_footer(&areas, buf, store, sid.as_ref());

        // BUG-163: the focused pane paints the live composer (inline
        // pause prompt OR spinner/transition/input — impl in
        // `input_area.rs`); every other pane paints a read-only ghost of
        // its session's persisted draft (or the dim placeholder).
        if pane.is_focused {
            self.paint_input_area(areas.input, buf, store, sid.as_ref());
            if let Some(p) = self.slash_popup.as_mut() {
                p.render(area, buf);
            } else if let Some(p) = self.file_popup.as_mut() {
                p.render(area, buf);
            }
            // RPC-382/383 + COPY-008: turn content modal overlay + selection.
            self.paint_turn_modal(area, buf, store);
        } else {
            self.paint_ghost_input_row(areas.input, buf, store, sid.as_ref());
        }
    }
}
