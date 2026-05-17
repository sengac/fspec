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
use crate::views::{AgentView, BoardView};

/// Which top-level view is currently visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Board,
    Agent,
}

/// Top-level navigator. Owns the BoardView + AgentView components; the
/// actual board/agent state lives on App via BoardStore +
/// AgentViewStore.
pub struct Navigator {
    pub board: BoardView,
    pub agent: AgentView,
    pub active_view: ViewMode,
    pub action_tx: Option<UnboundedSender<Action>>,
}

impl Navigator {
    pub fn new(theme: Arc<Theme>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            board: BoardView::new(theme, action_tx.clone()),
            agent: AgentView::new(action_tx.clone()),
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
    pub fn handle_event(
        &mut self,
        event: &Event,
        board_store: &BoardStore,
    ) -> EventResult {
        match self.active_view {
            ViewMode::Board => self.board.handle_event(event, board_store),
            ViewMode::Agent => self.agent.handle_event(event),
        }
    }

    /// React to a dispatched action that the App has already applied to
    /// the stores. The Navigator's only meaningful state is `active_view`.
    pub fn apply_action(&mut self, action: &Action) {
        match action {
            Action::EnterWorkUnit(_) | Action::OpenAgentView(_) => {
                self.active_view = ViewMode::Agent;
            }
            Action::BackToBoard => {
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

    fn render(
        nav: &mut Navigator,
        board: &BoardStore,
        agent: &mut AgentViewStore,
    ) -> String {
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
        assert!(out.contains("Agent"));
        assert!(out.contains("s-1"));
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
        nav.apply_action(&Action::OpenAgentView(None));
        assert_eq!(nav.active_view, ViewMode::Agent);
    }
}
