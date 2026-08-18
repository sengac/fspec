//! `App` struct + constructor + accessor surface (RPC-012, RPC-013).
//!
//! Holds the Compositor (modal layers only), Action bus, FspecBackend,
//! Theme, Navigator (BoardView + AgentView) and the BoardStore +
//! AgentViewStore. All store mutations happen synchronously inside
//! [`crate::app::dispatch`] on the App task (RPC-009 single-task).

use std::sync::Arc;

use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::components::Action;
use crate::compositor::Compositor;
use crate::mouse::clipboard::Osc52Clipboard;
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
    /// spawned by `App::bootstrap` per RPC-005 Q9.
    pub(crate) subscriber_tasks: Vec<JoinHandle<()>>,
    /// RPC-012 test-only seam: pending tasks spawned inside `App::dispatch`
    /// (e.g. lazy `create_session` on first `EnterWorkUnit`); tests use
    /// [`App::next_pending_task`] to await deterministically.
    pub(crate) pending_tasks: Vec<JoinHandle<()>>,
    /// Chunks-subscriber session filter (RPC-009 rule [8]). The subscriber
    /// task reads this before forwarding `Action::ChunkReceived`; `dispatch`
    /// republishes on `Action::SessionCreated`.
    pub(crate) active_session_tx: watch::Sender<Option<SessionId>>,
    pub(crate) active_session_rx: watch::Receiver<Option<SessionId>>,
    /// RPC-052: single in-flight debounced save handle for the per-session
    /// pending-input draft. `App::handle_pending_input_changed` aborts any
    /// previous handle then stores the fresh one so a second edit within the
    /// 300ms debounce window cancels the previous save.
    pub(crate) pending_input_save_handle: Option<JoinHandle<()>>,
    /// RPC-064: single in-flight debounced abort handle for the `/search`
    /// history-search round-trip. Aborts the previous handle so rapid
    /// keystrokes inside the 150ms debounce window collapse to a single
    /// `backend.persistence_search_history(query)` call.
    pub(crate) search_history_debounce_handle: Option<tokio::task::AbortHandle>,
    /// TUI-093: per-session guard recording which sessions have already had
    /// the persisted default thinking level applied (Rust equivalent of the
    /// TS `appliedToSessionRef`), so a manual `/thinking` pick is never
    /// clobbered when that session regains focus.
    pub(crate) applied_default_thinking: std::collections::HashSet<SessionId>,
    /// RPC-373: local port the RPC-372 viewer server bound to at bootstrap;
    /// `None` when start failed (the board `D` key then no-ops).
    pub(crate) viewer_port: Option<u16>,
    /// RPC-373: handle to the running viewer server, retained so it shuts down cleanly on App drop. `None` when the server failed to start.
    pub(crate) viewer_handle: Option<codelet_attachment_viewer::ViewerHandle>,
    /// COPY-006: OSC 52 clipboard writer (boxed; tests inject a Vec<u8>).
    pub(crate) clipboard: Osc52Clipboard<Box<dyn std::io::Write + Send>>,
    /// RPC-416: ORIGINATING session + stable scrollback seq of the live inline reconnect notice (replace/remove target this, not focus).
    pub(crate) reconnect_notice: Option<(SessionId, u64)>,
    /// RPC-416: auto-dismiss timer armed on `Reconnected`; aborted on a re-drop so a stale clear can't remove a fresh notice.
    pub(crate) reconnect_dismiss_handle: Option<JoinHandle<()>>,
    /// RPC-430: pre-session debug-capture toggle. When no session is
    /// active, `/debug` toggles this flag instead of calling the backend.
    /// On session creation the flag is propagated to the new session.
    pub(crate) pre_session_debug_enabled: bool,
    pub(crate) compaction_hide_handles: std::collections::HashMap<SessionId, JoinHandle<()>>, // RPC-417 auto-hide timers
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
            pending_input_save_handle: None,
            search_history_debounce_handle: None,
            applied_default_thinking: std::collections::HashSet::new(),
            viewer_port: None,
            viewer_handle: None,
            clipboard: Osc52Clipboard::new(Box::new(std::io::stdout())),
            reconnect_notice: None,
            reconnect_dismiss_handle: None,
            compaction_hide_handles: std::collections::HashMap::new(),
            pre_session_debug_enabled: false,
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

    /// TUI-106: the checkpoints view's list stage has flushed AND the
    /// view holds no checkpoints — the real "No checkpoints available"
    /// empty state may surface (loading ≠ empty discriminator).
    pub fn navigator_checkpoints_loaded_and_empty(&self) -> bool {
        self.navigator.checkpoints.load.is_loaded()
            && !self.navigator.checkpoints.load.is_loading()
            && self.navigator.checkpoints.is_empty()
    }

    /// TUI-106: the changed-files view's scan has flushed AND the view
    /// holds no files — the real "No changed files" empty state may
    /// surface (loading ≠ empty discriminator).
    pub fn navigator_changed_files_loaded_and_empty(&self) -> bool {
        self.navigator.changed_files.load.is_loaded()
            && !self.navigator.changed_files.load.is_loading()
            && self.navigator.changed_files.is_empty()
    }

    /// TUI-106: the checkpoints cascade's in-flight stage label ("Loading
    /// files for {name}…" / "Loading diff for {path}…"), or the list
    /// label while the list is loading; `None` once the cascade idles.
    pub fn navigator_checkpoints_active_label(&self) -> Option<String> {
        self.navigator.checkpoints.load.active_label()
    }

    /// TUI-109: a clone of the CheckpointsView's LoadingDialog (test
    /// seam for asserting the counter row the progress fold feeds).
    pub fn navigator_checkpoints_loading_dialog(&self) -> crate::components::loading_dialog::LoadingDialog {
        self.navigator.checkpoints.loading.clone()
    }

    /// TUI-109: the number of checkpoints folded into the
    /// CheckpointsView (test seam).
    pub fn navigator_checkpoints_len(&self) -> usize {
        self.navigator.checkpoints.checkpoints_len()
    }

    /// TUI-109: whether the CheckpointsView's list stage has flushed
    /// (test seam for the progress stale-drop guard).
    pub fn navigator_checkpoints_list_loaded(&self) -> bool {
        self.navigator.checkpoints.load.is_loaded()
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

    /// RPC-430: read the pre-session debug-capture toggle flag.
    pub fn pre_session_debug_enabled(&self) -> bool {
        self.pre_session_debug_enabled
    }

    /// RPC-430: set the pre-session debug-capture toggle flag.
    pub fn set_pre_session_debug_enabled(&mut self, val: bool) {
        self.pre_session_debug_enabled = val;
    }
}
