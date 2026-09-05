//! Navigator — top-level view that switches between BoardView and
//! AgentView. Feature files:
//! spec/features/rpc012-board-agent-navigation.feature,
//! spec/features/rpc013-source-shape.feature.
//!
//! Cards: RPC-012 (replaces RPC-009 `RootView`), RPC-013 (footer moved
//! into each view; Navigator hands the full area to the active child).
//! Renders EXACTLY ONE child view per frame — either BoardView OR
//! AgentView — over the full area. Each view paints its own 1-row
//! footer per RPC-013.

use std::sync::Arc;

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, EventResult, Priority};
use crate::store::{AgentViewStore, BoardStore};
use crate::theme::Theme;
use crate::views::multiplex::{
    keys as mux_keys, mouse as mux_mouse, render as mux_render, MultiplexLayout,
};
use crate::views::{
    AgentView, BlocklistView, BoardView, ChangedFilesView, CheckpointsView, ModelSelectorView,
    ProviderSettingsView,
};

/// Which top-level view is currently visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Board,
    Agent,
    /// RPC-054: ProviderSettingsView entered via the `/provider` slash
    /// command. Returns to `Agent` on Esc.
    ProviderSettings,
    /// RPC-056: BlocklistView entered via the `/blocklist` slash
    /// command. Returns to `Agent` on Esc.
    Blocklist,
    /// RPC-337: full-screen ModelSelectorView entered via `/model`
    /// (`OpenModelSelectorView`) or ProviderSettings Tab
    /// (`SwitchToModels`). Returns to `Agent` on Esc or after a model
    /// is committed.
    ModelSelector,
    /// RPC-356: dual-pane ChangedFilesView entered from the board via
    /// the `F` key (`OpenChangedFilesView`). Returns to `Board` on Esc
    /// or `CloseChangedFilesView`.
    ChangedFiles,
    /// RPC-364: three-pane CheckpointsView entered via the board `C` key.
    Checkpoints,
    /// MUX-001: multiplex grid of top-level views (Board | Agent |
    /// ChangedFiles | Checkpoints) entered via `/mux` or the `m` key.
    Mux,
}

/// Top-level navigator. Owns the BoardView + AgentView components; the
/// actual board/agent state lives on App via BoardStore +
/// AgentViewStore.
pub struct Navigator {
    pub board: BoardView,
    pub agent: AgentView,
    /// RPC-054: ProviderSettingsView owned by the Navigator alongside
    /// the existing children.
    pub provider_settings: ProviderSettingsView,
    /// RPC-056: BlocklistView owned by the Navigator alongside the
    /// existing children.
    pub blocklist: BlocklistView,
    /// RPC-337: full-screen ModelSelectorView owned by the Navigator.
    pub model_selector: ModelSelectorView,
    /// RPC-356: dual-pane ChangedFilesView owned by the Navigator.
    pub changed_files: ChangedFilesView,
    /// RPC-364: three-pane CheckpointsView owned by the Navigator.
    pub checkpoints: CheckpointsView,
    /// MUX-001: multiplex grid layout (config + cached pane rects +
    /// focus + divider drag state).
    pub mux: MultiplexLayout,
    pub active_view: ViewMode,
    pub action_tx: Option<UnboundedSender<Action>>,
}

impl Navigator {
    pub fn new(theme: Arc<Theme>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            board: BoardView::new(theme, action_tx.clone()),
            agent: AgentView::new(action_tx.clone()),
            provider_settings: ProviderSettingsView::new(),
            blocklist: BlocklistView::new(),
            model_selector: ModelSelectorView::new(),
            changed_files: ChangedFilesView::new(),
            checkpoints: CheckpointsView::new(),
            mux: MultiplexLayout::new(),
            active_view: ViewMode::Board,
            action_tx: Some(action_tx),
        }
    }

    pub fn priority(&self) -> Priority {
        Priority::Background
    }

    pub fn id(&self) -> &str {
        "navigator"
    }

    /// TUI-106: true iff the ACTIVE lazy mode-view (Checkpoints or
    /// Changed Files) has a cascade stage in flight. Mirrors how
    /// `App::is_input_animating` delegates to the owned agent view
    /// (`app/state.rs`); the run loop feeds this into the 4th
    /// `tick_should_draw` operand to keep the loading dialog's
    /// braille spinner animated.
    pub fn is_view_loading(&self) -> bool {
        match self.active_view {
            ViewMode::Checkpoints => self.checkpoints.is_loading(),
            ViewMode::ChangedFiles => self.changed_files.is_loading(),
            _ => false,
        }
    }

    /// MUX-006: true iff the mux focus flash is in flight — the mux
    /// view is active AND the layout has an armed flash inside its
    /// 350ms window. The run loop feeds this into the 5th
    /// `tick_should_draw` operand so the 16ms tick keeps redrawing the
    /// flash even when the session is idle (R6). With mux off (any
    /// single-view mode) this is always false (R7).
    pub fn is_mux_flash_active(&self) -> bool {
        self.active_view == ViewMode::Mux && self.mux.is_flash_active()
    }

    /// Route a keyboard or mouse event to the active sub-view. RPC-023
    /// extended this from `Event::Key`-only forwarding so the BoardView
    /// mouse-handling slice sees `Event::Mouse(_)` for wheel scroll and
    /// click-to-focus hit-testing.
    pub fn handle_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult {
        match self.active_view {
            ViewMode::Board => self.board.handle_event(event, board_store),
            ViewMode::Agent => self.agent.handle_event(event),
            ViewMode::ProviderSettings => self.handle_provider_settings_event(event),
            ViewMode::Blocklist => self.handle_blocklist_event(event),
            ViewMode::ModelSelector => self.handle_model_selector_event(event),
            ViewMode::ChangedFiles => self.handle_changed_files_event(event),
            ViewMode::Checkpoints => self.handle_checkpoints_event(event),
            ViewMode::Mux => self.handle_mux_event(event, board_store),
        }
    }

    /// MUX-001: route an event through the mux grid. Keyboard input
    /// goes to the focused pane ONLY (the isolation "trap"); mouse
    /// events hit-test the divider (drag) then the pane rects
    /// (click-to-focus + forward).
    fn handle_mux_event(&mut self, event: &Event, board_store: &BoardStore) -> EventResult {
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
                                == crate::views::multiplex::MuxOrientation::Horizontal;
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
        use crate::views::multiplex::{MuxOrientation, DIVIDER_SIZE};
        let horizontal = self.mux.config().orientation == MuxOrientation::Horizontal;
        let pane_start = self
            .mux
            .pane_rects()
            .get(index)
            .map(|r| if horizontal { r.x } else { r.y })
            .unwrap_or(0);
        let first =
            self.mux
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

    /// React to a dispatched action that the App has already applied to
    /// the stores. The Navigator's only meaningful state is `active_view`.
    /// RPC-097: `OpenAgentView(None)` MUST NOT flip the view — the
    /// dialog overlays BoardView; the view switch is deferred to
    /// `handle_create_session_submitted` on confirm.
    pub fn apply_action(&mut self, action: &Action) {
        match action {
            Action::EnterWorkUnit(_) | Action::OpenAgentView(Some(_)) => {
                self.active_view = ViewMode::Agent;
            }
            Action::OpenAgentView(None) => {}
            Action::BackToBoard => {
                // MUX-001: retain the mux grid when it is active —
                // "back to board" focuses the board pane within the
                // grid instead of flipping the whole view out of Mux.
                // BUG-175: gate on the LIVE view, not the persisted
                // `mux.config().enabled` flag (the App dispatch arm
                // applies the same rule first; this arm re-runs per
                // action, so it needs the identical guard). The flag
                // is a saved layout preference that survives restarts;
                // acting on it while the grid is not entered used to
                // strand BackToBoard as a no-op (session close from
                // single-view mode landed on a blank Agent).
                if self.active_view == ViewMode::Mux {
                    let board_idx = self
                        .mux
                        .effective_panes()
                        .iter()
                        .position(|k| *k == crate::views::multiplex::MuxPaneKind::Board)
                        .unwrap_or(0);
                    self.mux.set_focus(board_idx);
                } else {
                    self.active_view = ViewMode::Board;
                }
            }
            Action::OpenProviderSettingsView => {
                self.active_view = ViewMode::ProviderSettings;
            }
            Action::CloseProviderSettingsView => self.active_view = ViewMode::Agent,
            Action::OpenBlocklistView => self.active_view = ViewMode::Blocklist,
            Action::CloseBlocklistView => self.active_view = ViewMode::Agent,
            // RPC-337: model selector mode-view flips.
            Action::OpenModelSelectorView => {
                self.active_view = ViewMode::ModelSelector;
            }
            // Committing a model OR explicit close both return to Agent.
            Action::CloseModelSelectorView | Action::ModelSelected(..)
                if self.active_view == ViewMode::ModelSelector =>
            {
                tracing::info!(
                    target: "model_select",
                    "[MODEL-SELECT] navigator apply_action: closing ModelSelector view -> Agent"
                );
                self.active_view = ViewMode::Agent;
            }
            // RPC-356: changed-files mode-view flips.
            Action::OpenChangedFilesView => {
                self.active_view = ViewMode::ChangedFiles;
            }
            Action::CloseChangedFilesView if self.active_view == ViewMode::ChangedFiles => {
                self.active_view = ViewMode::Board;
            }
            // RPC-364: checkpoints mode-view flips.
            Action::OpenCheckpointsView => self.active_view = ViewMode::Checkpoints,
            Action::CloseCheckpointsView if self.active_view == ViewMode::Checkpoints => {
                self.active_view = ViewMode::Board;
            }
            // MUX-001: R8 — Enter on a board work unit in mux mode
            // focuses the agent pane WITHOUT flipping the whole view
            // (the board stays visible in its pane). All other mux
            // transitions are /mux-driven (dispatch_mux.rs).
            Action::MuxEnterWorkUnit(_) if self.active_view == ViewMode::Mux => {
                let agent_idx = self.mux.agent_pane_index(&self.mux.config().panes, 0);
                self.mux.set_focus(agent_idx);
            }
            _ => {}
        }
    }

    /// Render against the live stores. Caller is App.
    ///
    /// RPC-013: the active child receives the full `area` — the
    /// Navigator no longer reserves a 1-row footer chunk because each
    /// view now paints its own view-specific footer.
    pub fn render_with_stores(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        board_store: &BoardStore,
        agent_store: &mut AgentViewStore,
    ) {
        match self.active_view {
            ViewMode::Board => {
                self.board.render_with_store(area, buf, board_store);
            }
            ViewMode::Agent => {
                self.agent.render_with_store(area, buf, agent_store);
            }
            ViewMode::ProviderSettings => {
                self.provider_settings.render(area, buf);
            }
            ViewMode::Blocklist => {
                let empty = std::collections::HashSet::new();
                let disabled = agent_store
                    .current_session()
                    .and_then(|sid| agent_store.blocklist_disabled_for(sid))
                    .unwrap_or(&empty);
                self.blocklist.render(area, buf, disabled);
            }
            ViewMode::ModelSelector => {
                self.model_selector.render(area, buf);
            }
            ViewMode::ChangedFiles => {
                self.changed_files.render(area, buf);
            }
            ViewMode::Checkpoints => {
                self.checkpoints.render(area, buf);
            }
            ViewMode::Mux => {
                mux_render::render_with_stores(
                    &mut self.mux,
                    area,
                    buf,
                    board_store,
                    agent_store,
                    &mut mux_render::MuxRenderViews {
                        board: &self.board,
                        agent: &mut self.agent,
                        changed_files: &mut self.changed_files,
                        checkpoints: &mut self.checkpoints,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::store::{AgentViewStore, BoardStore};
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
