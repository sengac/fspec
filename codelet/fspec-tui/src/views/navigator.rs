//! Navigator — top-level view that switches between BoardView and
//! AgentView.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc013-source-shape.feature
//!
//! Cards: RPC-012 (replaces RPC-009 `RootView`), RPC-013 (footer moved
//!   into each view; Navigator hands the full area to the active child).
//!
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
use crate::views::{AgentView, BlocklistView, BoardView, ChangedFilesView, CheckpointsView, ModelSelectorView, ProviderSettingsView};

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
            Action::BackToBoard => self.active_view = ViewMode::Board,
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
}
