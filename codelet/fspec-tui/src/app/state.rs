//! `App` struct + constructor + accessor surface (RPC-012, RPC-013).
//!
//! Holds the Compositor (modal layers only — HelpDialog / DisconnectDialog),
//! the Action bus, the FspecBackend handle, the Theme, the Navigator
//! (which owns BoardView + AgentView, each painting its own footer per
//! RPC-013), and the two stores (BoardStore + AgentViewStore) that the
//! navigator's children read from.
//!
//! All store mutations happen synchronously inside [`crate::app::dispatch`]
//! on the App task per the RPC-009 single-task tenere pattern — no Mutex /
//! RwLock / AtomicXyz anywhere on the store surface.

use std::sync::Arc;

use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::components::Action;
use crate::compositor::Compositor;
use crate::store::{AgentViewStore, BoardStore};
use crate::theme::Theme;
use crate::transport::FspecBackend;
use crate::views::{Navigator, ViewMode};

/// Application root.
///
/// Compositor is reserved for MODAL layers (HelpDialog, DisconnectDialog).
/// The always-on background is the [`Navigator`] which paints either
/// `BoardView` or `AgentView` plus a 1-row footer per frame depending on
/// `Navigator.active_view`.
pub struct App {
    pub(crate) compositor: Compositor,
    pub(crate) action_tx: UnboundedSender<Action>,
    pub(crate) action_rx: UnboundedReceiver<Action>,
    pub(crate) backend: Arc<dyn FspecBackend>,
    pub(crate) theme: Arc<Theme>,
    pub(crate) navigator: Navigator,
    /// RPC-012: work-units state. Mutated only on the App task.
    pub(crate) board_store: BoardStore,
    /// RPC-012: agent-view session-navigation state. Mutated only on the
    /// App task.
    pub(crate) agent_view_store: AgentViewStore,
    pub(crate) should_quit: bool,
    pub(crate) should_render: bool,
    /// Subscriber-task handles (work_units_rx / chunks_rx / logs_rx),
    /// spawned by `App::bootstrap` via `tokio::spawn` on the host runtime
    /// per RPC-005 Q9. Aborted on Drop in a future card.
    pub(crate) subscriber_tasks: Vec<JoinHandle<()>>,
    /// RPC-012 test-only seam: pending tasks spawned inside
    /// `App::dispatch` (e.g. lazy `create_session` on first
    /// `EnterWorkUnit`). Production code never reads this; tests use
    /// [`App::next_pending_task`] to await deterministically.
    pub(crate) pending_tasks: Vec<JoinHandle<()>>,
    /// Chunks-subscriber session filter (RPC-009 rule [8]). The
    /// subscriber task reads the current value before forwarding
    /// `Action::ChunkReceived`; `App::dispatch` publishes a new value
    /// whenever `Action::SessionCreated` updates the AgentViewStore.
    pub(crate) active_session_tx: watch::Sender<Option<SessionId>>,
    pub(crate) active_session_rx: watch::Receiver<Option<SessionId>>,
}

impl App {
    /// Construct an App against `backend` with an empty Compositor.
    pub fn new(backend: Arc<dyn FspecBackend>) -> Self {
        let (action_tx, action_rx) = unbounded_channel();
        Self::with_action_bus(backend, action_tx, action_rx)
    }

    /// RPC-011 rule [21]: construct an App with an externally-owned
    /// action bus so the transport-layer reconnect supervisor can share
    /// the App's `UnboundedSender<Action>` and publish
    /// `Action::Disconnected` / `Action::Reconnecting(n)` /
    /// `Action::Reconnected` directly onto the App's bus.
    pub fn with_action_bus(
        backend: Arc<dyn FspecBackend>,
        action_tx: UnboundedSender<Action>,
        action_rx: UnboundedReceiver<Action>,
    ) -> Self {
        let theme = Arc::new(Theme::default());
        let navigator = Navigator::new(theme.clone(), action_tx.clone());
        let (active_session_tx, active_session_rx) = watch::channel(None);
        Self {
            compositor: Compositor::new(),
            action_tx,
            action_rx,
            backend,
            theme,
            navigator,
            board_store: BoardStore::default(),
            agent_view_store: AgentViewStore::default(),
            should_quit: false,
            should_render: true,
            subscriber_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            active_session_tx,
            active_session_rx,
        }
    }

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

    // ── RPC-009/RPC-011 legacy shims (migrated to the new stores) ───────

    /// Snapshot of work units currently in the BoardStore (legacy
    /// accessor used by RPC-009 tests). Returns a flat list in
    /// insertion order — the new code uses
    /// [`crate::store::BoardStore::column_units`] for column-grouped
    /// queries.
    pub fn work_units_snapshot(&self) -> Vec<codelet_rpc_types::WorkUnitInfo> {
        use crate::store::COLUMN_ORDER;
        let mut out = Vec::new();
        for column in COLUMN_ORDER {
            for unit in self.board_store.column_units(column) {
                out.push(unit.clone());
            }
        }
        out
    }

    /// AgentViewStore's current session id (legacy accessor used by
    /// RPC-009/RPC-011 tests).
    pub fn current_session(&self) -> Option<SessionId> {
        self.agent_view_store.current_session().cloned()
    }
}
