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
use crate::views::Navigator;

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
    /// MUX-001: mux config persistence state (shared fspec-config.json, tui.mux).
    pub(crate) mux_state: crate::store::MuxState,
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
            mux_state: crate::store::MuxState::new(),
        }
    }
}

