//! Component trait + Priority enum + EventResult + Action types.
//!
//! Feature: spec/features/fspec-tui-trait-surface.feature
//! Rules: [5] Priority, [6] EventResult, [7] Component
//!
//! These types are deliberately small and free of UI logic — they sit
//! beneath the [`crate::compositor::Compositor`] dispatcher and are the
//! only thing the App layer ever sees of a "widget".

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::compositor::Compositor;

pub mod disconnect_dialog;
pub mod hello;
pub mod help_dialog;

/// Event-handling priority for layered components (RPC-002 doc 09 §A.1).
///
/// `#[repr(u32)]` with discriminants from RPC-008 rule [5]. The exact
/// numeric values matter for cross-binary stability and for the
/// dispatcher's `sort_by_key` ordering.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Background = 100,
    Low = 200,
    Medium = 500,
    High = 800,
    Critical = 1000,
}

/// Deferred mutation of the [`Compositor`] requested by a layer's
/// [`Component::handle_event`] return value (RPC-008 rule [6]).
///
/// The closure receives `&mut Compositor` AFTER dispatch completes so
/// the layer can request its own removal — or push a new layer — without
/// running into borrow conflicts with the iterator dispatch was using.
pub type Callback = Box<dyn FnOnce(&mut Compositor) + Send>;

/// Outcome of a [`Component::handle_event`] call (RPC-008 rule [6]).
///
/// `Ignored` propagates to the next layer in priority order. `Consumed`
/// short-circuits dispatch. Both variants may carry an optional
/// [`Callback`] that the App layer runs against the compositor AFTER
/// dispatch unwinds.
pub enum EventResult {
    Ignored(Option<Callback>),
    Consumed(Option<Callback>),
}

impl EventResult {
    /// Convenience helper: a no-callback Ignored.
    pub fn ignored() -> Self {
        EventResult::Ignored(None)
    }

    /// Convenience helper: a no-callback Consumed.
    pub fn consumed() -> Self {
        EventResult::Consumed(None)
    }

    /// Returns true iff this result short-circuited dispatch.
    pub fn is_consumed(&self) -> bool {
        matches!(self, EventResult::Consumed(_))
    }
}

/// Application-level action propagated through every layer's
/// [`Component::update`] (RPC-008 rule [10]).
///
/// In RPC-008 the only Action that matters end-to-end is `Quit` —
/// later cards (RPC-009 list view + REPL, RPC-002 Slice 03+ widgets)
/// will extend this enum.
///
/// RPC-009 extension: seven new variants threaded through the App's
/// bootstrap subscriber tasks. The `PartialEq, Eq` derives from RPC-008
/// are dropped because `StreamChunk` (carried by `ChunkReceived`) does
/// not derive `PartialEq` (it transitively contains struct types like
/// `TokenTracker` and `ToolCallInfo` that only derive `Debug, Clone,
/// Serialize, Deserialize`). Tests that need to inspect Action values
/// use `matches!()` rather than `assert_eq!()`.
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    /// Render request — typically emitted after a state change.
    Redraw,
    /// Future-proofing for tests + later cards: a string-tagged action
    /// the App fans out across the compositor without interpreting.
    Custom(String),
    /// RPC-009: trigger a fresh `backend.list_work_units()` snapshot
    /// fetch (used by the work_units subscriber task on
    /// `RecvError::Lagged`).
    LoadWorkUnits,
    /// RPC-009: a fresh work-units snapshot has arrived (either from
    /// bootstrap's `list_work_units()` or from the broadcast subscriber
    /// task converting `work_units_rx` messages).
    WorkUnitsLoaded(Vec<codelet_rpc_types::WorkUnitInfo>),
    /// RPC-009: bootstrap's `create_session(None)` returned a session id;
    /// the AgentReplView records it as its active session.
    SessionCreated(codelet_rpc_types::SessionId),
    /// RPC-009: a streaming chunk arrived for the (session_id, chunk)
    /// pair. The chunks_rx subscriber filters by the AgentReplView's
    /// active session id BEFORE emitting this variant.
    ChunkReceived(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk),
    /// RPC-009: the user pressed Enter on the AgentReplView's input
    /// box. The App's run loop dispatches `backend.send_input` and then
    /// forwards into compositor.update so layers can react.
    InputSubmitted(String),
    /// RPC-009: the user pressed Ctrl+C on the AgentReplView's input
    /// box. The App's run loop dispatches `backend.interrupt` —
    /// Ctrl+C is NOT a quit (Ctrl+D / `q` are).
    Interrupt,
    /// RPC-009: the user pressed Tab; the App's run loop mutates
    /// RootView's `focused_pane` field to alternate between WorkUnits
    /// and Repl.
    FocusNext,
    /// RPC-011 CR-1: the WebSocketFspecBackend has detected its
    /// underlying WS stream has dropped. Emitted onto the action bus
    /// by the transport-layer supervisor task (rule [0] / [18]).
    /// App.dispatch() pushes a DisconnectDialog @ Priority::Critical
    /// onto the Compositor in response.
    Disconnected,
    /// RPC-011 auto-reconnect: the supervisor task is about to attempt
    /// connect_async for the Nth time (1-indexed). The DisconnectDialog
    /// re-renders its body to "auto-reconnecting (attempt N)…".
    Reconnecting(u32),
    /// RPC-011 auto-reconnect: the supervisor successfully reconnected
    /// AND re-issued bootstrap (list_work_units + create_session(None)
    ///   + resubscribed three broadcasts). App.dispatch() pops the
    ///     DisconnectDialog from the Compositor.
    Reconnected,
    /// RPC-011 CR-1: the user pressed 'r' while DisconnectDialog was
    /// topmost. The supervisor cancels its current backoff sleep, tries
    /// connect immediately, and resets the backoff schedule.
    ManualReconnect,
    /// RPC-012: BoardView emits this when the user presses Enter on a
    /// selected work unit. App::dispatch sets
    /// AgentViewStore.current_work_unit_id + status and switches the
    /// Navigator's active_view to Agent.
    EnterWorkUnit(String),
    /// RPC-012: BoardView emits this when the user presses Shift+Right.
    /// `Some(sid)` requests navigation to the attached session;
    /// `None` requests the create-session dialog.
    OpenAgentView(Option<codelet_rpc_types::SessionId>),
    /// RPC-012: AgentView emits this on ESC. Navigator flips
    /// active_view back to Board.
    BackToBoard,
    /// RPC-012: programmatic navigation-target set/clear (used by
    /// future slices to script navigation without a keystroke).
    NavigationTargetSet(Option<codelet_rpc_types::SessionId>),
    /// RPC-012: explicit session-attachment action. App::dispatch
    /// emits this on Action::SessionCreated when
    /// AgentViewStore.current_work_unit_id is Some(_).
    AttachSession(String, codelet_rpc_types::SessionId),
    /// RPC-012: BoardView navigation — focus previous column.
    FocusPrevColumn,
    /// RPC-012: BoardView navigation — focus next column.
    FocusNextColumn,
    /// RPC-012: BoardView navigation — move selection down in focused
    /// column.
    SelectNext,
    /// RPC-012: BoardView navigation — move selection up in focused
    /// column.
    SelectPrev,
    /// RPC-012: placeholder — priority reorder up. Backend persistence
    /// is out of scope for this slice (no-op in App::dispatch).
    ReorderUp,
    /// RPC-012: placeholder — priority reorder down.
    ReorderDown,
    /// RPC-015: bootstrap's `backend.checkpoint_counts()` has returned;
    /// the BoardStore's `checkpoint_counts` field is updated so the
    /// BoardView header paints the live counts on the next render.
    CheckpointCountsLoaded(codelet_rpc_types::CheckpointCounts),
    /// RPC-016: PageUp pressed while BoardView is focused. The payload
    /// is the most recent viewport_height observed by BoardView so
    /// App::dispatch can scroll the focused column by exactly that
    /// many rows (matching the TS UnifiedBoardLayout behaviour).
    ScrollFocusedColumnUp(usize),
    /// RPC-016: PageDown pressed while BoardView is focused. Same
    /// payload semantics as `ScrollFocusedColumnUp`.
    ScrollFocusedColumnDown(usize),
    /// RPC-016: Home pressed while BoardView is focused. App::dispatch
    /// resets the focused column's selected_index + scroll_offset to 0.
    SelectFirstInFocused,
    /// RPC-016: End pressed while BoardView is focused. App::dispatch
    /// sets the focused column's selected_index to units.len()-1 and
    /// adjusts the scroll_offset so the last unit stays visible.
    SelectLastInFocused,
    /// RPC-023: BoardView emits this on a left-click hit-test against
    /// a column header (or content row). `App::dispatch` maps the index
    /// through `COLUMN_ORDER` and calls `BoardStore::set_focused_column`.
    SetFocusedColumn(usize),
    /// RPC-023: BoardView emits this on a left-click hit-test against
    /// a content row. The payload is the WORK-UNIT index in the focused
    /// column (visible row + `scroll_offset`). App::dispatch routes
    /// through `BoardStore::select_index_in_focused` so the click both
    /// updates the selection AND adjusts the viewport when crossing the
    /// scroll boundary.
    SelectIndexInFocused(usize),
    /// RPC-023: scaffolding for the TUI-078 native-text-selection
    /// toggle (RPC-019). Emitted by `MouseTrackingToggle`'s tokio timer
    /// 5 seconds after the most recent `temporarily_disable` call. The
    /// payload is the toggle's `owner_id` so multiple toggle instances
    /// can coexist (a future App::dispatch routing layer will look up
    /// the owner and call `re_enable()`). For RPC-023 this variant
    /// exists but `App::dispatch` does not yet route it — the BoardView
    /// slice intentionally does not opt into TUI-078 button-press.
    ReEnableMouseTracking(String),
    /// RPC-018: per-session ModelInfo arrived (typically from a spawned
    /// `backend.get_model_info(session)` task fired in response to
    /// `Action::SessionCreated`). App::dispatch writes the payload
    /// into `AgentViewStore.model_info_by_session[session]`.
    ModelInfoLoaded(codelet_rpc_types::SessionId, codelet_rpc_types::ModelInfo),
    /// RPC-018: per-session ThinkingLevel arrived. Sibling of
    /// `ModelInfoLoaded` — fired by the same `Action::SessionCreated`
    /// dispatch arm.
    ThinkingLevelLoaded(codelet_rpc_types::SessionId, codelet_rpc_types::ThinkingLevel),
    /// RPC-018: workspace snapshot arrived (typically from
    /// `App::bootstrap` firing `backend.get_workspace_info()`).
    /// App::dispatch writes the payload into `AgentViewStore.workspace`.
    WorkspaceInfoLoaded(codelet_rpc_types::WorkspaceInfo),
    /// RPC-019: AgentView emits this when the user presses Shift+Up
    /// inside the MultiLineInput. RPC-021 will wire App::dispatch
    /// routing to walk the command history backward.
    HistoryPrev,
    /// RPC-019: AgentView emits this when the user presses Shift+Down.
    /// Sibling of `HistoryPrev` — walks forward through command history.
    HistoryNext,
    /// RPC-019: AgentView emits this when the user presses Shift+Left.
    /// RPC-021 will route through App::dispatch to cycle to the
    /// previous session in the AgentViewStore's session list.
    SessionPrev,
    /// RPC-019: AgentView emits this when the user presses Shift+Right.
    /// Sibling of `SessionPrev` — cycles forward through sessions.
    SessionNext,
    /// RPC-020: AgentView emits this when the user picks a command
    /// from the slash palette (Enter on a highlighted row). App::dispatch
    /// branches per `SlashCommandAction` variant — Help pushes the
    /// HelpDialog onto the Compositor, Clear resets the scrollback,
    /// Quit sets `should_quit`, all others surface a `[notice]`
    /// scrollback line until the future RPC card lands them.
    SlashCommandSelected(crate::views::agent::slash_commands::SlashCommandAction),
    /// RPC-020: AgentView emits this after the user types into the
    /// `@<filter>` token. App::dispatch spawns a tokio task calling
    /// `backend.search_files(prefix, 20)` and emits
    /// `Action::FileSearchResults(matches)` on success.
    SearchFiles(String),
    /// RPC-020: backend search returned this list of paths — App::dispatch
    /// forwards into AgentView's file_popup via `set_matches`.
    FileSearchResults(Vec<String>),
    /// RPC-024: AgentView emits this when the user presses PageUp.
    /// App::dispatch routes the scroll into the currently-focused
    /// SessionContext's ScrollbackList so cross-session scroll state
    /// stays correctly per-session.
    ScrollbackPageUp,
    /// RPC-024: sibling of `ScrollbackPageUp` — emitted on PageDown /
    /// End. App::dispatch advances the focused SessionContext's
    /// scrollback offset, snapping back to stick-mode at the tail.
    ScrollbackPageDown,
    /// RPC-025: emitted by the spawned `backend.persistence_get_history`
    /// task fired in response to the user's first `Action::HistoryPrev`
    /// on a session. Carries the SessionId and the newest-first list of
    /// past submitted inputs. App::dispatch caches the snapshot, snaps
    /// `recall_index` to `Some(0)`, and replaces the MultiLineInput
    /// value with `snapshot[0]`. Dropped (no-op) when the snapshot is
    /// empty so HistoryPrev with no history leaves state untouched.
    HistorySnapshotLoaded(codelet_rpc_types::SessionId, Vec<String>),
    /// RPC-026: open the /resume session picker. App::dispatch installs
    /// AgentView.resume_popup AND spawns `backend.list_sessions()`.
    OpenResumePicker,
    /// RPC-026: open the /search history palette. App::dispatch
    /// installs AgentView.search_popup with an empty query (no
    /// backend call yet — that fires on the first SearchHistory).
    OpenSearchPalette,
    /// RPC-026: a spawned `backend.list_sessions()` task resolved.
    /// App::dispatch folds the result into the open resume_popup via
    /// `set_sessions`. No-op when resume_popup is None.
    SessionListLoaded(Vec<codelet_rpc_types::SessionInfo>),
    /// RPC-026: emitted by the resume picker on Enter. App::dispatch
    /// either moves `current_session_index` to the matching slot in
    /// AgentViewStore.open_sessions OR appends a fresh
    /// SessionContext::new(id) if the session is not yet open. Also
    /// publishes to active_session_tx and runs refresh_session_chrome.
    AttachToSession(codelet_rpc_types::SessionId),
    /// RPC-026: emitted by the search palette as the user types.
    /// App::dispatch spawns `backend.persistence_search_history(q)` and
    /// dispatches Action::HistorySearchResults on success.
    SearchHistory(String),
    /// RPC-026: a spawned `backend.persistence_search_history` task
    /// resolved. App::dispatch folds the result into the open
    /// search_popup via `set_matches`. No-op when search_popup is None.
    HistorySearchResults(Vec<codelet_rpc_types::HistoryMatch>),
    /// RPC-026: emitted by the search palette on Enter with the
    /// highlighted match's text. App::dispatch replaces the
    /// MultiLineInput value with `text` AND drops the search_popup.
    /// Does NOT auto-submit — the user may edit before pressing Enter.
    InsertIntoInput(String),
}

/// Visible UI element that participates in event dispatch + rendering.
///
/// Deliberately `Send` (no `Sync`) so layers can hold non-Sync state
/// (e.g. once-locks, channels) — the [`Compositor`] is single-threaded
/// from a Component's perspective.
pub trait Component: Send {
    /// Event-dispatch priority (default `Medium`).
    fn priority(&self) -> Priority {
        Priority::Medium
    }

    /// When false the layer is skipped during dispatch + render
    /// (default true).
    fn is_active(&self) -> bool {
        true
    }

    /// Stable identifier — used by `Compositor::remove` and tests.
    /// No default; every Component must provide one.
    fn id(&self) -> &str;

    /// Handle a crossterm `Event`. Default: `Ignored(None)` — the
    /// dispatcher then walks down to the next layer.
    fn handle_event(&mut self, _event: &Event) -> EventResult {
        EventResult::ignored()
    }

    /// Receive an [`Action`] fanned out top-down. Default: `None`.
    fn update(&mut self, _action: Action) -> Option<Action> {
        None
    }

    /// Paint the layer onto the supplied [`Buffer`]. No default —
    /// every Component must implement.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
