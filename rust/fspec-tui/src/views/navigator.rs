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
use crate::views::multiplex::{render as mux_render, MultiplexLayout};
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
