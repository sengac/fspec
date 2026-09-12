//! Navigator — MUX-001 grid event routing (extracted from `navigator.rs`
//! so that file stays under the 300-LoC ceiling pinned by
//! `source_shape_rpc013`).
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Methods are `pub(crate)` so the sibling `navigator` module can call
//! them from `Navigator::handle_event`.

use crossterm::event::Event;

use crate::components::{Action, EventResult};
use crate::store::BoardStore;
use crate::views::multiplex::{keys as mux_keys, mouse as mux_mouse, MuxOrientation};

use super::navigator::Navigator;

impl Navigator {
    /// MUX-001: route an event through the mux grid. Keyboard input
    /// goes to the focused pane ONLY (the isolation "trap"); mouse
    /// events hit-test the divider (drag) then the pane rects
    /// (click-to-focus + forward).
    pub(crate) fn handle_mux_event(
        &mut self,
        event: &Event,
        board_store: &BoardStore,
    ) -> EventResult {
        let is_mouse = matches!(event, Event::Mouse(_));
        if is_mouse {
            let decision = mux_mouse::classify_mouse(&self.mux, event);
            return match decision {
                mux_mouse::MouseDecision::DividerDown { index } => {
                    self.mux.begin_drag(index);
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::DividerDrag { index } => {
                    if self.mux.is_dragging {
                        if let Some((col, row)) = mux_mouse::mouse_pos(event) {
                            let (pos, total) = self.mux_drag_axis(col, row, index);
                            let horizontal = self.mux.config().orientation
                                == MuxOrientation::Horizontal;
                            let cursor = if horizontal { col } else { row };
                            // BUG-166: live width = cursor minus the
                            // DRAGGED pane's origin (the drag tracks the
                            // cursor); the stored percent is relative to
                            // that pane's width over the available axis,
                            // so release keeps the divider in place.
                            let pane_start = self
                                .mux
                                .pane_rects()
                                .get(index)
                                .map(|r| if horizontal { r.x } else { r.y })
                                .unwrap_or(0);
                            let width = cursor.saturating_sub(pane_start);
                            self.mux.update_drag(index, width, pos, total);
                        }
                    }
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::DividerUp { .. } => {
                    self.mux.finish_drag();
                    EventResult::consumed()
                }
                mux_mouse::MouseDecision::Pane { index } => {
                    self.mux.set_focus(index);
                    let result = self.forward_mux_event_to_focused_pane(event, board_store);
                    if result.is_consumed() {
                        result
                    } else {
                        EventResult::consumed()
                    }
                }
                mux_mouse::MouseDecision::Gap => EventResult::ignored(),
            };
        }
        // BUG-179: a bracketed paste (Event::Paste) — including a terminal
        // file drop delivered as a paste — honours the same keyboard
        // isolation as keys: it reaches the FOCUSED pane only. The focused
        // pane's handler decides: the Agent pane's paste precedence/gate
        // chain (HITL / exec-stdin / compacting / CRLF) inserts into the
        // live composer; the Board/Files/Checkpoints panes ignore it.
        // Pre-fix the Event::Key guard below dropped every non-key event
        // here, so the paste was silently lost.
        if matches!(event, Event::Paste(_)) {
            return self.forward_mux_event_to_focused_pane(event, board_store);
        }
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        let key = *key;
        // R8: Enter on the focused BOARD pane in mux mode binds the
        // selected work unit + focuses the agent pane WITHOUT flipping
        // the whole view. Intercepted here (before the board handler
        // would emit EnterWorkUnit).
        if key.code == crossterm::event::KeyCode::Enter
            && self.mux.focus() < self.mux.effective_panes().len()
            && self.mux.effective_panes()[self.mux.focus()]
                == crate::views::multiplex::MuxPaneKind::Board
        {
            if let Some(unit) = board_store.selected_work_unit() {
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(Action::MuxEnterWorkUnit(unit.id.clone()));
                }
                return EventResult::consumed();
            }
        }
        let decision = mux_keys::classify_key(&self.mux, &key);
        match decision {
            mux_keys::KeyDecision::FocusPrev => {
                // MUX-002: agent window backward-rotation OR focus
                // movement (stops at the first pane — no wrap).
                self.mux.shift_left();
                EventResult::consumed()
            }
            mux_keys::KeyDecision::FocusNext => {
                // MUX-002: agent window forward-rotation, focus
                // movement, or a new-agent prompt at the right edge.
                if self.mux.shift_right() {
                    self.emit_mux_new_agent();
                }
                EventResult::consumed()
            }
            mux_keys::KeyDecision::Forward => {
                // Forward to the focused pane; if the pane ignores the
                // key, fall through to the App-level shortcuts (e.g.
                // '?' help, 'm' mux toggle) — mirroring the
                // single-view cascade.
                self.forward_mux_event_to_focused_pane(&Event::Key(key), board_store)
            }
        }
    }

    /// MUX-002: open the CreateSessionDialog (no work-unit attachment)
    /// for a Shift+Right new-agent prompt at the right mux edge.
    fn emit_mux_new_agent(&self) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(Action::OpenCreateSessionDialog { preselect: None });
        }
    }

    /// The (dragged-pane width at the cursor, available axis span) for a
    /// divider drag. BUG-166: the stored percent is the dragged pane's
    /// width over the AVAILABLE axis (panes + dividers subtracted) —
    /// the same basis the layout math uses — so release keeps the
    /// divider within one cell of where the user left it.
    fn mux_drag_axis(&self, col: u16, row: u16, index: usize) -> (u16, u16) {
        use crate::views::multiplex::DIVIDER_SIZE;
        let horizontal = self.mux.config().orientation == MuxOrientation::Horizontal;
        let pane_start = self
            .mux
            .pane_rects()
            .get(index)
            .map(|r| if horizontal { r.x } else { r.y })
            .unwrap_or(0);
        let first = self
            .mux
            .pane_rects()
            .first()
            .map_or(pane_start, |r| if horizontal { r.x } else { r.y });
        let last_end = self.mux.pane_rects().last().map_or(pane_start + 1, |r| {
            if horizontal {
                r.x + r.width
            } else {
                r.y + r.height
            }
        });
        let body = last_end.saturating_sub(first);
        let n = self.mux.pane_rects().len();
        let available = body
            .saturating_sub((n.saturating_sub(1)) as u16 * DIVIDER_SIZE)
            .max(1);
        let cursor = if horizontal { col } else { row };
        (cursor.saturating_sub(pane_start), available)
    }

    /// Forward an event to the mux's currently-focused pane (keyboard
    /// isolation: unfocused panes receive NO events).
    fn forward_mux_event_to_focused_pane(
        &mut self,
        event: &Event,
        board_store: &BoardStore,
    ) -> EventResult {
        let focus = self.mux.focus();
        let kind = self
            .mux
            .effective_panes()
            .get(focus)
            .copied()
            .unwrap_or_default();
        if focus >= self.mux.effective_panes().len() {
            return EventResult::consumed();
        }
        mux_keys::forward_to_pane(
            event,
            board_store,
            &self.board,
            &mut self.agent,
            &mut self.changed_files,
            &mut self.checkpoints,
            kind,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::sync::Arc;

    use crate::components::Action;
    use crate::store::{AgentViewStore, BoardStore};
    use crate::theme::Theme;
    use crate::views::navigator::ViewMode;
    use crate::views::Navigator;
    use codelet_rpc_types::{SessionId, WorkUnitInfo};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use tokio::sync::mpsc::unbounded_channel;

    fn wu(id: &str, status: &str) -> WorkUnitInfo {
        WorkUnitInfo {
            id: id.to_string(),
            title: id.to_string(),
            work_type: "story".to_string(),
            status: status.to_string(),
            description: None,
            estimate: None,
            epic: None,
            attachments: Vec::new(),
            last_state_change_at: None,
        }
    }

    fn fresh() -> (Navigator, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = unbounded_channel();
        (Navigator::new(Arc::new(Theme::default()), tx), rx)
    }

    fn render(nav: &mut Navigator, board: &BoardStore, agent: &mut AgentViewStore) -> String {
        let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
        term.draw(|frame| {
            nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
        })
        .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }
        joined
    }

    #[test]
    fn renders_board_when_active_view_is_board() {
        let (mut nav, _rx) = fresh();
        let mut board = BoardStore::default();
        board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
        let mut agent = AgentViewStore::default();
        let out = render(&mut nav, &board, &mut agent);
        assert!(out.contains("BACKLOG"));
        assert!(out.contains("SPECIFYING"));
        assert!(out.contains("AUTH-001"));
    }

    #[test]
    fn renders_agent_when_active_view_is_agent() {
        let (mut nav, _rx) = fresh();
        nav.active_view = ViewMode::Agent;
        let board = BoardStore::default();
        let mut agent = AgentViewStore::default();
        agent.append_session(crate::store::SessionContext::new(SessionId::new("s-1")));
        let out = render(&mut nav, &board, &mut agent);
        // RPC-029: scrollback no longer paints an " Agent — s-1 " title;
        // the only header-side anchor for AgentView is the input
        // placeholder hint (or the empty header itself).
        assert!(out.contains("Type a message..."));
        assert!(!out.contains("BACKLOG"));
    }

    #[test]
    fn apply_action_flips_view_mode() {
        let (mut nav, _rx) = fresh();
        assert_eq!(nav.active_view, ViewMode::Board);
        nav.apply_action(&Action::EnterWorkUnit("AUTH-001".to_string()));
        assert_eq!(nav.active_view, ViewMode::Agent);
        nav.apply_action(&Action::BackToBoard);
        assert_eq!(nav.active_view, ViewMode::Board);
        // RPC-097: OpenAgentView(None) must NOT flip — dialog overlays board.
        nav.apply_action(&Action::OpenAgentView(None));
        assert_eq!(
            nav.active_view,
            ViewMode::Board,
            "OpenAgentView(None) must keep view on Board"
        );
        // OpenAgentView(Some(_)) DOES flip — jumping into existing session.
        nav.apply_action(&Action::OpenAgentView(Some(
            codelet_rpc_types::SessionId::new("s-1"),
        )));
        assert_eq!(nav.active_view, ViewMode::Agent);
    }

    // ── BUG-164: BackToBoard must retain the active mux grid ──────────────

    /// Feature: spec/features/rust-mux-mode.feature
    /// Scenario: closing a session in mux mode retains the mux and focuses the board pane
    #[test]
    fn back_to_board_in_mux_retains_the_grid_and_focuses_the_board_pane() {
        let (mut nav, _rx) = fresh();
        nav.mux.enable_default();
        nav.active_view = ViewMode::Mux;
        nav.mux.set_focus(1); // agent pane
                              // @step When BackToBoard lands while the mux grid is active
        nav.apply_action(&Action::BackToBoard);
        // @step Then the view stays in Mux (no single-view flip to Board)
        assert_eq!(
            nav.active_view,
            ViewMode::Mux,
            "BackToBoard must NOT flip the whole view out of the mux grid"
        );
        // @step And the Board pane is focused within the grid
        let panes = nav.mux.effective_panes();
        assert_eq!(
            panes[nav.mux.focus()],
            crate::views::multiplex::MuxPaneKind::Board,
            "BackToBoard must focus the Board pane inside the grid"
        );
    }

    /// Feature: spec/features/rust-mux-mode.feature
    /// Scenario: existing single-view behavior is unchanged when mux is off
    #[test]
    fn back_to_board_outside_mux_still_flips_to_the_board_view() {
        let (mut nav, _rx) = fresh();
        nav.active_view = ViewMode::Agent;
        assert!(!nav.mux.config().enabled);
        nav.apply_action(&Action::BackToBoard);
        assert_eq!(
            nav.active_view,
            ViewMode::Board,
            "BackToBoard with mux inactive must flip to the single Board view"
        );
    }
}
