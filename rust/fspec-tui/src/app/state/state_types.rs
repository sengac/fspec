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
    /// RPC-448: the daemons the board switcher offers (installed by the binary).
    pub(crate) daemon_directory: Option<Arc<dyn crate::daemons::DaemonDirectory>>,
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
    pub(crate) pending_creation_context: Option<codelet_rpc_types::WorkUnitContext>,
    /// TUI-124: the facts the open close dialog states — what confirming
    /// closes (None while no close dialog is open).
    pub(crate) pending_close: Option<crate::components::exit_confirmation_dialog::CloseFacts>,
    pub(crate) completed_story_creations: std::collections::HashSet<SessionId>,
    pub(crate) session_storage: crate::app::session_storage::SessionStorageState,
    pub(crate) model_sync:
        std::collections::HashMap<SessionId, crate::app::dispatch_model_sync::ModelSync>,
    /// BUG-576: the session created for no-session model selections.
    pub(crate) no_session_selection:
        Option<crate::app::dispatch_model_no_session::NoSessionSelection>,
    pub(crate) continue_sync:
        std::collections::HashMap<SessionId, crate::app::dispatch_continue_sync::ContinueSync>,
    pub(crate) transcript_sync:
        std::collections::HashMap<SessionId, crate::app::transcript_sync::TranscriptSync>,
    /// Chunks-subscriber session filter (RPC-009 rule [8]). The subscriber
    /// task reads this before forwarding `Action::ChunkReceived`; `dispatch`
    /// republishes on `Action::SessionCreated`.
    pub(crate) active_session_tx: watch::Sender<Option<SessionId>>,
    pub(crate) active_session_rx: watch::Receiver<Option<SessionId>>,
    /// BUG-549: the session whose draft the shared composer holds, plus a
    /// per-session ordered draft-save slot (`app/composer_draft.rs`).
    pub(crate) composer_drafts: crate::app::composer_draft::ComposerDrafts,
    /// BUG-575: the number of the last Provider Settings refresh requested.
    pub(crate) provider_settings_refreshes: u64,
    /// RPC-064: single in-flight debounced abort handle for the `/search`
    /// history-search round-trip. Aborts the previous handle so rapid
    /// keystrokes inside the 150ms debounce window collapse to a single
    /// `backend.persistence_search_history(query)` call.
    pub(crate) search_history_debounce_handle: Option<tokio::task::AbortHandle>,
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
    /// BOARD-024: card-details attachment workspace, launcher and viewers.
    pub(crate) board_card: crate::app::board_card_attachments::BoardCardRuntime,
    /// TUI-126: the confirmation of the latest copy.
    pub(crate) copy_notice: crate::app::copy_notice::CopyNotice,
    /// TUI-126: opens URLs in the browser (tests record them).
    pub(crate) url_launcher: crate::app::url_launcher::UrlLauncher,
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
            daemon_directory: None,
            theme,
            navigator,
            board_store: BoardStore::default(),
            agent_view_store: AgentViewStore::default(),
            should_quit: false,
            should_render: true,
            subscriber_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            pending_creation_context: None,
            pending_close: None,
            completed_story_creations: std::collections::HashSet::new(),
            session_storage: Default::default(),
            model_sync: std::collections::HashMap::new(),
            no_session_selection: None,
            continue_sync: std::collections::HashMap::new(),
            transcript_sync: std::collections::HashMap::new(),
            active_session_tx,
            active_session_rx,
            composer_drafts: Default::default(),
            provider_settings_refreshes: 0,
            search_history_debounce_handle: None,
            clipboard: Osc52Clipboard::new(Box::new(std::io::stdout())),
            reconnect_notice: None,
            reconnect_dismiss_handle: None,
            compaction_hide_handles: std::collections::HashMap::new(),
            pre_session_debug_enabled: false,
            mux_state: crate::store::MuxState::new(),
            board_card: Default::default(),
            copy_notice: Default::default(),
            url_launcher: crate::app::url_launcher::system_launcher(),
        }
    }
}
