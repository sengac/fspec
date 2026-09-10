//! `App` accessor + run-loop surface (RPC-012, RPC-013 and successors).
//!
//! Compositor / backend / theme accessors, action-bus helpers, the
//! run-loop flags, store + navigator accessors, legacy shims and the
//! MUX-001 persistence hooks. The struct + constructors live in
//! [`super::state_types`].

use std::sync::Arc;

use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::components::Action;
use crate::compositor::Compositor;
use crate::store::{AgentViewStore, BoardStore};
use crate::theme::Theme;
use crate::transport::FspecBackend;
use crate::views::{Navigator, ViewMode};

use super::state_types::App;

impl App {
    // ── Compositor + backend + theme accessors ──────────────────────────

    /// Borrow the Compositor (modal layers only).
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    /// Mutably borrow the Compositor (for tests).
    pub fn compositor_mut(&mut self) -> &mut Compositor {
        &mut self.compositor
    }

    /// Borrow the backend handle (for tests).
    pub fn backend(&self) -> &Arc<dyn FspecBackend> {
        &self.backend
    }

    /// Borrow the shared Theme palette.
    pub fn theme(&self) -> &Arc<Theme> {
        &self.theme
    }

    // ── Run-loop flags ──────────────────────────────────────────────────

    /// True iff `q` / Ctrl+D has fired.
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// True iff a render is pending.
    pub fn should_render(&self) -> bool {
        self.should_render
    }

    /// Mark a render as having been served.
    pub fn mark_rendered(&mut self) {
        self.should_render = false;
    }

    // ── Action bus ──────────────────────────────────────────────────────

    /// Send an [`Action`] onto the App's bus.
    pub fn send_action(&self, action: Action) -> anyhow::Result<()> {
        self.action_tx
            .send(action)
            .map_err(|e| anyhow::anyhow!("action bus closed: {e}"))
    }

    /// RPC-011 rule [21]: expose a clone of the bus sender so the
    /// transport-layer reconnect supervisor can emit lifecycle actions
    /// without holding a reference to the App itself.
    pub fn action_tx_clone(&self) -> UnboundedSender<Action> {
        self.action_tx.clone()
    }

    /// RPC-026 test-only seam: borrow a clone of the currently-active
    /// session id as published on the `active_session_tx` watch
    /// channel. Used by App-dispatch tests to assert that
    /// `Action::AttachToSession` republished the new SessionId.
    pub fn active_session_rx_snapshot(&self) -> Option<SessionId> {
        self.active_session_rx.borrow().clone()
    }

    /// RPC-093: lookup the current session's status (Running /
    /// Compacting / Idle / etc) from the AgentViewStore. Returns
    /// `None` if no session is active.
    pub fn current_session_status(&self) -> Option<codelet_rpc_types::SessionStatus> {
        let sid = self.agent_view_store.current_session()?;
        self.agent_view_store.session_status_for(sid).copied()
    }

    /// RPC-093 rule [6]: true iff the current session is Running or
    /// Compacting — drives the run-loop "redraw every tick" bypass so
    /// the spinner advances even without inbound chunks.
    pub fn is_session_busy(&self) -> bool {
        matches!(
            self.current_session_status(),
            Some(codelet_rpc_types::SessionStatus::Running)
                | Some(codelet_rpc_types::SessionStatus::Compacting)
        )
    }

    /// RPC-093: true iff the AgentView input row is mid-finish-animation
    /// (Hiding/Showing). Plumbed into `tick_should_draw` so the
    /// run loop keeps drawing every 16ms tick even AFTER the session
    /// has gone Idle, letting the 5 char/17ms sweep complete.
    pub fn is_input_animating(&self) -> bool {
        self.navigator.agent.is_input_animating()
    }

    /// TUI-106: true iff the active lazy mode-view (Checkpoints or
    /// Changed Files) has a cascade stage in flight. Plumbed as the
    /// 4th operand of [`crate::app::tick_should_draw`] so the 16ms
    /// tick keeps redrawing the animated loading dialog.
    pub fn is_view_loading(&self) -> bool {
        self.navigator.is_view_loading()
    }

    /// Drain a single Action from the bus (test helper).
    pub fn try_recv_action(&mut self) -> Option<Action> {
        self.action_rx.try_recv().ok()
    }

    /// RPC-009 test accessor: number of subscriber tasks alive.
    pub fn subscriber_task_count(&self) -> usize {
        self.subscriber_tasks.len()
    }

    // ── RPC-012 store + navigator accessors ─────────────────────────────

    /// Borrow the BoardStore.
    pub fn board_store(&self) -> &BoardStore {
        &self.board_store
    }

    /// Mutably borrow the BoardStore (tests + `dispatch`).
    pub fn board_store_mut(&mut self) -> &mut BoardStore {
        &mut self.board_store
    }

    /// Borrow the AgentViewStore.
    pub fn agent_view_store(&self) -> &AgentViewStore {
        &self.agent_view_store
    }

    /// Mutably borrow the AgentViewStore.
    pub fn agent_view_store_mut(&mut self) -> &mut AgentViewStore {
        &mut self.agent_view_store
    }

    /// Borrow the Navigator (BoardView + AgentView container; each
    /// child paints its own footer per RPC-013).
    pub fn navigator(&self) -> &Navigator {
        &self.navigator
    }

    /// Mutably borrow the Navigator.
    pub fn navigator_mut(&mut self) -> &mut Navigator {
        &mut self.navigator
    }

    /// Current top-level view (Board or Agent).
    pub fn active_view(&self) -> ViewMode {
        self.navigator.active_view
    }

    /// RPC-012 test-only seam: pop the most recently spawned tokio task
    /// so a test can await it deterministically (e.g. lazy session
    /// creation on first `Action::EnterWorkUnit`).
    pub fn next_pending_task(&mut self) -> Option<JoinHandle<()>> {
        self.pending_tasks.pop()
    }
}
