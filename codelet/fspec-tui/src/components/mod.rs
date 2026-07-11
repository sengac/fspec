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

pub mod attachment_picker_dialog;
pub mod board_exit_confirmation_dialog;
pub mod checkpoint_restore_dialog;
pub mod create_session_dialog;
pub mod dialog_theme;
pub mod dialog_theme_rows;
pub mod disconnect_dialog;
pub mod error_dialog;
pub mod exit_confirmation_dialog;
pub mod hello;
pub mod help_content;
pub mod help_dialog;
pub mod help_dialog_scroll;
pub mod list_scrollbar;
pub mod model_selector_dialog_rows;
pub mod notification_dialog;
pub mod role_dialog;
pub mod scroll_viewport;
pub mod status_dialog;
pub mod thinking_level_dialog;

/// Event-handling priority for layered components (RPC-002 doc 09 §A.1).
///
/// `#[repr(u32)]` with discriminants from RPC-008 rule [5]. The exact
/// numeric values matter for cross-binary stability and for the
/// dispatcher's `sort_by_key` ordering.
///
/// RPC-022 added [`Priority::Foreground`] (900) between `High` and
/// `Critical` so the new modal dialogs (ModelSelectorDialog,
/// ThinkingLevelDialog) sit above the always-on background views but
/// still beneath `Critical` dialogs (HelpDialog / DisconnectDialog).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Background = 100,
    Low = 200,
    Medium = 500,
    High = 800,
    Foreground = 900,
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
    /// PROV-101: `create_session` returned an EMPTY `SessionId` because no
    /// default model was set (decline). The TUI must NOT append an empty-id
    /// session — every caller maps the empty id to THIS action via
    /// `app::session_creation::post_create_session_action`, and App::dispatch
    /// surfaces it as a Priority::Critical ErrorDialog so the decline is
    /// explicit instead of being silently swallowed.
    SessionCreationDeclined,
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
    /// AND re-issued bootstrap (list_work_units + create_session(None)).
    /// RPC-415: App.dispatch() also respawns the five broadcast subscriber
    /// tasks (work_units / chunks / logs / status_changes / session_created)
    /// bound to the new client's receivers, then pops the DisconnectDialog
    /// from the Compositor.
    Reconnected,
    /// RPC-011 CR-1: the user pressed 'r' while DisconnectDialog was
    /// topmost. The supervisor cancels its current backoff sleep, tries
    /// connect immediately, and resets the backoff schedule.
    ManualReconnect,
    /// RPC-416: fired by the auto-dismiss timer armed on `Reconnected`.
    /// Carries the ORIGINATING session id + the stable scrollback `seq`
    /// of the inline reconnect notice so `App::dispatch` removes exactly
    /// that chunk (silent no-op if the session closed or the notice was
    /// already superseded by a re-drop).
    ClearReconnectNotice {
        /// The session the reconnect notice was pushed into.
        session_id: codelet_rpc_types::SessionId,
        /// The stable scrollback seq of the notice chunk to remove.
        seq: u64,
    },
    /// RPC-417: 10s auto-hide fired — seq-guarded COMPACTED badge clear.
    ClearCompactionReduction {
        session_id: codelet_rpc_types::SessionId,
        seq: u64,
    },
    /// RPC-012: BoardView emits this on Enter on a selected work unit.
    /// App::dispatch sets AgentViewStore.current_work_unit_id + status
    /// and switches the Navigator's active_view to Agent.
    EnterWorkUnit(String),
    /// RPC-012: BoardView emits this on Shift+Right. `Some(sid)` = navigate
    /// to the attached session; `None` = create-session dialog.
    OpenAgentView(Option<codelet_rpc_types::SessionId>),
    /// RPC-012: AgentView emits this on ESC — Navigator flips back to Board.
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
    ThinkingLevelLoaded(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::ThinkingLevel,
    ),
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
    /// RPC-094: emitted by `views/agent/dispatch.rs` when the user
    /// presses the Up arrow AND `MultiLineInput::handle_event` returns
    /// `Ignored` because the cursor is on the first visual line. The
    /// App routes this into the focused SessionContext, dropping
    /// stick-to-bottom and decrementing offset by exactly 1.
    ScrollbackLineUp,
    /// RPC-094: sibling of `ScrollbackLineUp` — emitted by the Down
    /// arrow at the last visual line of the input buffer.
    ScrollbackLineDown,
    /// RPC-094: emitted on Home when the input does not consume it
    /// (empty buffer or cursor already at position 0). Jumps the
    /// focused SessionContext's scrollback offset to 0 and drops
    /// stick-to-bottom.
    ScrollbackHome,
    /// RPC-094: emitted by `views/agent/mouse_dispatch.rs` when a
    /// `MouseEventKind::ScrollUp` hits inside the scrollback rect.
    /// The `u32` carries the velocity multiplier (1..=5) sampled from
    /// the AgentView's `WheelVelocity` primitive.
    ScrollbackMouseWheelUp(u32),
    /// RPC-094: sibling of `ScrollbackMouseWheelUp` — emitted on
    /// `MouseEventKind::ScrollDown` inside the scrollback rect.
    ScrollbackMouseWheelDown(u32),
    /// RPC-025: emitted by the spawned `backend.persistence_get_history`
    /// task fired in response to the user's first `Action::HistoryPrev`
    /// on a session. Carries the SessionId and the newest-first list of
    /// past submitted inputs. App::dispatch caches the snapshot, snaps
    /// `recall_index` to `Some(0)`, and replaces the MultiLineInput
    /// value with `snapshot[0]`. Dropped (no-op) when the snapshot is
    /// empty so HistoryPrev with no history leaves state untouched.
    HistorySnapshotLoaded(codelet_rpc_types::SessionId, Vec<String>),
    /// RPC-026: open the /resume session picker mode view. App::dispatch
    /// installs AgentView.resume_view AND spawns `backend.list_sessions()`.
    OpenResumeView,
    /// RPC-026: open the /search history palette mode view.
    /// App::dispatch installs AgentView.search_view with an empty query
    /// (no backend call yet — that fires on the first SearchHistory).
    OpenSearchView,
    /// RPC-026: drop the resume view without changing focus. Emitted by
    /// the resume view's Esc handler (Dismiss outcome).
    CloseResumeView,
    /// RPC-026: drop the search view without inserting anything.
    /// Emitted by the search view's Esc handler (Dismiss outcome).
    CloseSearchView,
    /// RPC-026: a spawned `backend.list_sessions()` task resolved.
    /// App::dispatch folds the result into the open resume_view via
    /// `set_sessions`. No-op when resume_view is None.
    SessionListLoaded(Vec<codelet_rpc_types::SessionInfo>),
    /// RPC-026: emitted by the resume view on Enter. App::dispatch
    /// either moves `current_session_index` to the matching slot in
    /// AgentViewStore.open_sessions OR appends a fresh
    /// SessionContext::new(id) if the session is not yet open. Also
    /// publishes to active_session_tx and runs refresh_session_chrome.
    AttachToSession(codelet_rpc_types::SessionId),
    /// RPC-026: emitted by the search view as the user types.
    /// App::dispatch spawns `backend.persistence_search_history(q)` and
    /// dispatches Action::HistorySearchResults on success.
    SearchHistory(String),
    /// RPC-026: a spawned `backend.persistence_search_history` task
    /// resolved. App::dispatch folds the result into the open
    /// search_view via `set_matches`. No-op when search_view is None.
    ///
    /// RPC-064: widened to a struct variant carrying both the originating
    /// `query` and the `matches`. The dispatcher folds the matches ONLY
    /// when `query` still matches the live `search_view.query()` so
    /// stale responses for older queries are silently discarded.
    HistorySearchResults {
        query: String,
        matches: Vec<codelet_rpc_types::HistoryMatch>,
    },
    /// RPC-026: emitted by the search view on Enter with the
    /// highlighted match's text. App::dispatch replaces the
    /// MultiLineInput value with `text` AND drops the search_view.
    /// Does NOT auto-submit — the user may edit before pressing Enter.
    InsertIntoInput(String),
    /// RPC-026: emitted by the resume view when the user presses D on
    /// a row. App::dispatch surfaces a ConfirmDialog inside the
    /// resume_view — no backend call fires yet.
    RequestDeleteSession(codelet_rpc_types::SessionId),
    /// RPC-026: emitted by the ConfirmDialog when the user activates
    /// the primary "Delete" button. App::dispatch spawns
    /// `backend.persistence_delete_session(id)` and follows up with a
    /// fresh `backend.list_sessions()` → Action::SessionListLoaded
    /// so the resume view repaints without the deleted session.
    ConfirmDeleteSession(codelet_rpc_types::SessionId),
    /// RPC-022: emitted by the slash command popup (on Enter over the
    /// Thinking row) AND by `parse_slash_command("/thinking")`.
    /// App::dispatch pushes a fresh ThinkingLevelDialog onto the
    /// Compositor at Priority::Foreground seeded with the cached
    /// thinking level for the focused session.
    OpenThinkingDialog,
    /// RPC-022: a spawned `backend.list_providers()` task resolved.
    /// App::dispatch folds the result into the open ModelSelector
    /// mode-view. No-op when the view is not active.
    ListProvidersLoaded(Vec<codelet_rpc_types::ProviderInfo>),
    /// RPC-022: emitted by the ModelSelector mode-view on Enter over a
    /// model row. App::dispatch spawns `backend.set_session_model(...)` then
    /// re-runs `backend.get_model_info(...)` to refresh the
    /// SessionHeader chrome via `Action::ModelInfoLoaded`.
    /// PROV-117: the session id is `Option` — TS parity, the Enter handler
    /// has no session guard and always emits a selection; the backend write
    /// is skipped (and the selector still closes) when there is no session.
    ModelSelected(Option<codelet_rpc_types::SessionId>, String, String),
    /// RPC-022: emitted by ThinkingLevelDialog on Enter over a level
    /// row. App::dispatch spawns `backend.set_thinking_level(...)`
    /// then re-runs `backend.get_thinking_level(...)` to refresh the
    /// `[T:Level]` badge via `Action::ThinkingLevelLoaded`.
    ThinkingLevelSelected(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::ThinkingLevel,
    ),
    /// RPC-022: emitted by `parse_slash_command("/role …")` and by
    /// the bare `/role clear` path. App::dispatch updates
    /// AgentViewStore.role_by_session AND spawns
    /// `backend.set_session_role(...)`.
    SetSessionRole(codelet_rpc_types::SessionId, Option<String>),
    /// RPC-022: a spawned `backend.get_session_role(...)` task
    /// resolved. App::dispatch folds the result into
    /// AgentViewStore.role_by_session so the RoleBanner repaints. No
    /// backend write is fired for this — it is purely a read path.
    SessionRoleLoaded(codelet_rpc_types::SessionId, Option<String>),
    /// RPC-027: emitted by ThinkingLevelDialog on the `D` / `d` key.
    /// App::dispatch spawns `backend.set_thinking_level_default(...)`.
    /// The dialog stays open; no badge refresh fires because the
    /// default does not change the current session's effective level.
    SetThinkingLevelDefault(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::ThinkingLevel,
    ),
    /// RPC-045: emitted by the status_changes_rx subscriber when the
    /// SessionManager broadcasts a (SessionId, SessionStatus) transition.
    /// App::dispatch writes the payload into
    /// `AgentViewStore.session_status_by_session` so the SessionFooter
    /// status pill repaints push-driven on the next render — replacing
    /// the polling `get_session_status` path that previously refreshed
    /// the chrome.
    SessionStatusChanged(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::SessionStatus,
    ),
    /// RPC-046: emitted by spawned tokio tasks (currently the /clear
    /// `backend.clear_history` round-trip in `dispatch_slash_commands`) to
    /// route a `[notice]` / `[error]` scrollback line back to the
    /// ORIGINATING session — regardless of which session is currently
    /// focused when the response arrives. App::dispatch handles this
    /// by looking up the SessionContext for the supplied id and
    /// pushing the text via `SessionContext::push_line`. Silently
    /// drops when the session has been closed in the meantime.
    EmitSessionNotice(codelet_rpc_types::SessionId, String),
    /// RPC-049: emitted by `handle_attach_to_session`'s spawned
    /// `backend.resume_session(id)` task on the Ok branch. App::dispatch
    /// routes this into `handle_session_resume_complete`, which spawns
    /// a second task to call `backend.get_buffered_output(id, 1000)`
    /// and replays each returned chunk as `Action::ChunkReceived(id,
    /// chunk)` — so the resumed session's scrollback is seeded from
    /// the backend's replay buffer.
    SessionResumeComplete(codelet_rpc_types::SessionId),
    /// RPC-050: emitted by BoardView's Enter handler (and the explicit
    /// "attach" path) to bind the supplied work-unit id to the
    /// currently-focused AgentView session. App::dispatch routes this
    /// through `handle_attach_work_unit_to_session`, which spawns
    /// `backend.set_work_unit_context(session_id, Some(ctx))` and on
    /// Ok dispatches `Action::WorkUnitAttached`. With no current
    /// session the helper is a silent no-op.
    AttachWorkUnitToSession(String),
    /// RPC-050: emitted by the spawned `set_work_unit_context(Some)`
    /// task on the Ok branch. App::dispatch routes this through
    /// `handle_work_unit_attached`, which folds the supplied
    /// `WorkUnitContext` into `AgentViewStore.work_unit_context_by_session`
    /// so the SessionHeader chip and downstream per-session readers see
    /// the binding on the next frame.
    WorkUnitAttached(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::WorkUnitContext,
    ),
    /// RPC-050: emitted by the spawned `/detach` task on the Ok branch.
    /// App::dispatch routes this through `handle_work_unit_detached`,
    /// which clears the per-session binding in AgentViewStore, resets
    /// the focused session's scrollback (TS prepareForNewSession
    /// parity), and resets the per-session TokenState.
    WorkUnitDetached(codelet_rpc_types::SessionId),
    /// RPC-051: emitted by the AgentView's default Esc arm (after the
    /// popup → compositor-dialog → mode-view cascade levels have
    /// already had a chance to consume the keypress). App::dispatch
    /// routes this through `handle_agent_esc_pressed`, which:
    ///   - if the current session has SessionStatus::Running or
    ///     SessionStatus::Compacting → spawn `backend.interrupt(id)`
    ///     and stay on the AgentView (no navigation), matching the TS
    ///     Ink AgentView's Esc behaviour;
    ///   - otherwise → dispatch Action::BackToBoard.
    ///
    /// When no current session is open this is a fast path back to
    /// the Board view (silent no-op for the interrupt branch).
    AgentEscPressed,
    /// RPC-098: emitted by [`exit_confirmation_dialog::ExitConfirmationDialog`]
    /// when the user picks one of the three options (Detach / Close Session
    /// / Cancel). Routed through `App::dispatch` to `handle_agent_exit_choice`:
    ///   - `ExitChoice::Cancel` → no-op (dialog already removed via Callback);
    ///   - `ExitChoice::Detach` → dispatch `Action::BackToBoard` (the session
    ///     stays alive in the backend, mirroring the TS GlobalSessionStreamManager
    ///     detach semantics);
    ///   - `ExitChoice::CloseSession` → spawn `backend.destroy_session(id)` as
    ///     a pending task, then dispatch `Action::BackToBoard`.
    AgentExitChoice {
        choice: exit_confirmation_dialog::ExitChoice,
    },
    /// RPC-052: emitted by the AgentView's `views/agent/dispatch.rs`
    /// after `MultiLineInput::handle_event` returns Continued AND the
    /// buffer text actually changed (before/after value comparison).
    /// App::dispatch routes this through `handle_pending_input_changed`,
    /// which:
    ///   - mirrors the new text into the current SessionContext's
    ///     `input_draft` synchronously (no backend round-trip on
    ///     Shift+Left/Right cycling — RPC-024 fast path preserved);
    ///   - aborts any in-flight `pending_input_save_handle`;
    ///   - spawns a fresh task that sleeps 300ms then calls
    ///     `backend.set_pending_input(session, Some(text))`. Errors
    ///     are swallowed via `tracing` — no scrollback notice.
    ///
    /// With no current session this is a silent no-op.
    PendingInputChanged(String),
    /// RPC-052: emitted by the spawned `spawn_hydrate_pending_input`
    /// task on session activation when `backend.get_pending_input(id)`
    /// returns `Ok(Some(text))`. App::dispatch routes this through
    /// `handle_seed_pending_input`, which:
    ///   - always folds the text into the matching
    ///     `SessionContext.input_draft` (so the next cycle restores
    ///     the same draft);
    ///   - ONLY seeds the live `MultiLineInput` when the activated
    ///     session is still the focused session at the moment the
    ///     hydration completes (race-safe — late results targeting a
    ///     stale session are dropped from the live input).
    SeedPendingInput {
        session_id: codelet_rpc_types::SessionId,
        text: String,
    },
    /// RPC-053: emitted by the chunk dispatcher when
    /// `StreamChunk::SessionStateChange { state: Paused }` arrives.
    /// App::dispatch routes this through `handle_pause_chunk`, which
    /// spawns parallel `backend.get_pause_state(id)` and
    /// `backend.get_hitl_request(id)` reads. The first non-None result
    /// dispatches `Action::PauseStateFetched` (PauseState → RPC-406
    /// inline prompt slot) or `Action::HitlPromptFetched` (HitlRequest
    /// → RPC-411 inline prompt slot). When both return Some the HITL
    /// prompt wins (the HITL handler in the agent loop is the only
    /// path that populates `hitl_request`).
    PauseChunkReceived(codelet_rpc_types::SessionId),
    /// RPC-053: emitted by the chunk dispatcher when
    /// `StreamChunk::SessionStateChange { state: Running | Idle }`
    /// arrives. App::dispatch routes this through
    /// `handle_pause_cleared`, which clears the RPC-406 per-session
    /// pause slot AND the RPC-411 HITL slot so the UI does not
    /// strand a stale prompt after the agent loop has resumed
    /// server-side.
    PauseCleared(codelet_rpc_types::SessionId),
    /// RPC-406: emitted by `handle_pause_chunk` on `Ok(Some(state))`.
    /// App::dispatch stores the fetched PauseState into the
    /// AgentViewStore per-session pause slot — the AgentView paints
    /// the inline tool-approval prompt from that slot when the paused
    /// session is focused (replaces the deleted RPC-053 modal push).
    PauseStateFetched {
        session_id: codelet_rpc_types::SessionId,
        state: codelet_rpc_types::PauseState,
    },
    /// RPC-406: emitted by the AgentView pause-prompt key handler on
    /// ←/→ while a Triple pause prompt is showing. App::dispatch
    /// cycles the per-session selection with wraparound
    /// (0 = Allow Once, 1 = Allow Session, 2 = Deny).
    PausePromptNav {
        session_id: codelet_rpc_types::SessionId,
        delta: i32,
    },
    /// RPC-406: emitted by the AgentView pause-prompt key handler on
    /// Enter while a Triple pause prompt is showing. App::dispatch
    /// reads the authoritative selection from the store, maps it onto
    /// `ApprovalChoice::{Approve, ApproveSession, Deny}`, and routes
    /// through `handle_pause_triple` (which clears the slot).
    PausePromptEnter {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-411: emitted by `handle_pause_chunk` on
    /// `Ok(Some(hitl_request))`. App::dispatch stores the fetched
    /// request into the AgentViewStore per-session HITL slot — the
    /// AgentView paints the inline HITL prompt from that slot when the
    /// session is focused (replaces the deleted RPC-053 modal push).
    HitlPromptFetched {
        session_id: codelet_rpc_types::SessionId,
        request: codelet_rpc_types::HitlRequest,
    },
    /// RPC-411: ↑/↓ on an options question — cycle the selection with
    /// wraparound over `options.len() + 1` (virtual "Other...").
    HitlPromptNav {
        session_id: codelet_rpc_types::SessionId,
        delta: i32,
    },
    /// RPC-411: Enter on an options question. The reducer reads the
    /// authoritative slot state: Other... selected → enter Other mode,
    /// else capture `{id, selected:[label]}` and advance-or-submit.
    HitlPromptEnter {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-411: Enter in freeform/Other mode with non-empty text —
    /// the key handler reads + clears the SHARED composer input and
    /// carries the value here so the reducer stays store-authoritative.
    /// Captures `{id, selected:[], other:text}` and advance-or-submits.
    HitlAnswerCaptured {
        session_id: codelet_rpc_types::SessionId,
        text: String,
    },
    /// RPC-411: Esc in Other mode — local only: back to the options
    /// list, clears the empty-submit hint. NOTHING is sent.
    HitlOtherExit {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-411: empty/whitespace Enter in freeform/Other mode — sets
    /// the yellow empty-submit hint. NOTHING is sent.
    HitlEmptySubmit {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-411: typing in freeform/Other mode while the empty-submit
    /// hint is showing — clears the hint (useHitlInput.ts:201-208).
    HitlHintCleared {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-411: Esc outside Other mode — cancel the WHOLE request:
    /// sends `HitlResponse { cancelled: true, answers: [] }` via the
    /// backend, THEN clears the slot (no path clears without sending).
    HitlCancelled {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-053: emitted by the RPC-406 inline pause prompt (Confirm
    /// kind) on Y/N/Esc. App::dispatch routes this through
    /// `handle_pause_confirmed`, which clears the per-session pause
    /// slot and fires `backend.pause_confirm(session_id, accept)`
    /// fire-and-forget (best-effort UX so a slow backend does not
    /// leave the prompt dangling on screen).
    PauseConfirmed {
        session_id: codelet_rpc_types::SessionId,
        accept: bool,
    },
    /// RPC-053: emitted by the RPC-406 inline pause prompt (Triple
    /// kind) — via the `PausePromptEnter` reducer or directly with
    /// `ApprovalChoice::Deny` on Esc. App::dispatch routes this
    /// through `handle_pause_triple`, which clears the per-session
    /// pause slot and fires `backend.pause_triple(session_id, choice)`
    /// fire-and-forget.
    PauseTriple {
        session_id: codelet_rpc_types::SessionId,
        choice: codelet_rpc_types::ApprovalChoice,
    },
    /// RPC-053: routes through `handle_pause_resumed`, which fires
    /// `backend.pause_resume(session_id)` fire-and-forget. The agent
    /// loop receives the resume signal and unblocks from
    /// `wait_for_pause_response`. RPC-406: NOT reachable from the
    /// inline pause prompt (Esc DENIES) — kept for other callers
    /// (Continue-kind internal pauses via the napi path).
    PauseResumed {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-053/RPC-411: routes through `handle_hitl_submitted`, which
    /// fires `backend.send_hitl_response(session_id, response)`
    /// fire-and-forget and clears the per-session HITL slot.
    HitlSubmitted {
        session_id: codelet_rpc_types::SessionId,
        response: codelet_rpc_types::HitlResponse,
    },

    // ========================================================================
    // RPC-054: Provider settings view actions. The `/provider` slash
    // command dispatches `OpenProviderSettingsView`; the view itself
    // emits `SaveProviderCredentials` / `TestProviderConnection` /
    // `RefreshProviderModels` / `DeleteProviderCredentials`; the App
    // folds backend responses into the view via the *Loaded /
    // *Complete / *Refreshed variants.
    // ========================================================================
    /// RPC-054: emitted by the slash command palette (and `/provider`
    /// submit-line parser) to open the ProviderSettingsView. App::dispatch
    /// flips the Navigator's ViewMode to ProviderSettings AND spawns
    /// `backend.list_provider_credentials()`.
    OpenProviderSettingsView,
    /// RPC-054: emitted by the view on Esc (list mode). App::dispatch
    /// returns the Navigator to the Agent view.
    CloseProviderSettingsView,
    /// RPC-337: open the full-screen ModelSelector mode-view. Emitted
    /// by the `/model` slash command AND by ProviderSettings Tab
    /// (SwitchToModels). App::dispatch flips the Navigator's ViewMode to
    /// ModelSelector AND spawns `backend.list_providers()` (folded back
    /// via `Action::ListProvidersLoaded`). This is the only model-picker
    /// entry point (no Compositor modal exists).
    OpenModelSelectorView,
    /// RPC-337: emitted by the ModelSelector mode-view on Esc.
    /// App::dispatch returns the Navigator to the Agent view. A
    /// committed `Action::ModelSelected` ALSO returns to Agent (the
    /// selection closes the selector — handled in Navigator::apply_action).
    CloseModelSelectorView,
    /// RPC-337: emitted by the ModelSelector mode-view on `r`. Re-spawns
    /// `backend.list_providers()` so the provider/model tree refreshes;
    /// the view shows `(refreshing...)` until `ListProvidersLoaded`
    /// arrives.
    RefreshModelSelector,
    /// RPC-347: emitted by the ModelSelector mode-view's `a` keybind (wired in
    /// RPC-344) to create a NEW custom model on the focused profile.
    /// App::dispatch spawns `backend.add_custom_model(provider_id,
    /// profile_name, definition)` followed by a `list_providers` refresh.
    /// Added here as inert wire surface so the backend path exists before the
    /// keybinds/form land.
    AddCustomModel {
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::CustomModelDefinition,
    },
    /// RPC-347: emitted by the ModelSelector `e` keybind (RPC-344) to edit an
    /// existing custom model in place. `original_model_id` names the entry
    /// being replaced. App::dispatch spawns `backend.update_custom_model(..)`.
    EditCustomModel {
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: codelet_rpc_types::CustomModelDefinition,
    },
    /// RPC-347: emitted by the ModelSelector `d` keybind (RPC-344) after the
    /// delete-confirm overlay. App::dispatch spawns
    /// `backend.delete_custom_model(..)`.
    DeleteCustomModel {
        provider_id: String,
        profile_name: String,
        model_id: String,
    },
    /// RPC-054: a spawned `backend.list_provider_credentials()` task
    /// resolved. App::dispatch folds the result into the open
    /// ProviderSettingsView.
    ProviderCredentialsLoaded(Vec<codelet_rpc_types::ProviderCredentialInfo>),
    /// RPC-054: emitted by the view on Enter inside the edit-API-key
    /// form. App::dispatch spawns `backend.set_provider_credentials`
    /// followed by a fresh `list_provider_credentials` refresh.
    SaveProviderCredentials {
        provider_id: String,
        api_key: String,
    },
    /// RPC-054: emitted by the view on the `t` key. App::dispatch
    /// spawns `backend.test_provider_connection` and folds the result
    /// back via `Action::ProviderTestComplete`.
    TestProviderConnection(String),
    /// RPC-054: a spawned `backend.test_provider_connection` task
    /// resolved. App::dispatch folds the result into the view's status
    /// area.
    ProviderTestComplete {
        provider_id: String,
        result: codelet_rpc_types::TestConnectionResult,
    },
    /// RPC-054: emitted by the view on the `r` key. App::dispatch
    /// spawns `backend.refresh_models_cache` and follows up with a
    /// `list_provider_credentials` refresh.
    RefreshProviderModels(String),
    /// RPC-054: a spawned `backend.refresh_models_cache` task resolved.
    /// App::dispatch folds the new model count into the view via a
    /// follow-up list refresh and updates the status area.
    ProviderModelsRefreshed {
        provider_id: String,
        model_count: u32,
    },
    /// RPC-054: emitted by the view on the `d` key. App::dispatch
    /// spawns `backend.delete_provider_credentials` followed by a
    /// fresh `list_provider_credentials` refresh.
    DeleteProviderCredentials(String),
    /// RPC-054: emitted by the view AFTER the user accepts the
    /// ConfirmDialog primary button — replaces the previous
    /// direct-from-`d` flow so the destructive backend call never
    /// fires without explicit confirmation. App::dispatch routes this
    /// arm to the same `delete_provider_credentials` handler as the
    /// raw `DeleteProviderCredentials` variant.
    ConfirmDeleteProviderCredentials(String),
    /// RPC-054: emitted by the App for inline status updates that
    /// don't fit one of the typed *Loaded / *Complete variants
    /// (e.g. error messages from save / delete spawns).
    ProviderSettingsStatus(String),

    /// PROV-109: emitted by the profile create/edit form on submit.
    /// App::dispatch spawns `backend.save_profile(provider_id,
    /// profile_name, definition)` followed by a `list_provider_credentials`
    /// refresh so the openai profile slice repaints with the new state.
    /// PROV-136: `old_profile_name` is `Some(original)` for an edit-mode
    /// rename (dispatch routes to `rename_profile`), else `None`.
    SaveProfile {
        provider_id: String,
        profile_name: String,
        old_profile_name: Option<String>,
        definition: codelet_rpc_types::ProfileDefinition,
    },
    /// PROV-109: raw delete request for a local-server profile. App::dispatch
    /// spawns `backend.delete_profile(..)` + a list refresh. The view-layer is
    /// responsible for opening the confirm dialog before this fires.
    DeleteProfile {
        provider_id: String,
        profile_name: String,
    },
    /// PROV-109: emitted AFTER the user accepts the delete ConfirmDialog
    /// primary button. Routes through the SAME delete handler as
    /// `DeleteProfile` so the destructive backend call never fires without
    /// explicit confirmation (mirrors `ConfirmDeleteProviderCredentials`).
    ConfirmDeleteProfile {
        provider_id: String,
        profile_name: String,
    },
    /// PROV-116: emitted from the SUCCESS branch of a profile delete (just
    /// before the reload) to record the parent provider the reload should
    /// re-focus. A failed delete never emits it, so the cursor never jumps.
    ProfileDeleteNavigate {
        provider_id: String,
    },

    /// PROV-112: emitted by the ProviderSettingsView's DisconnectOAuth
    /// confirm dialog when the user presses `y`/`Y`. App::dispatch spawns
    /// `backend.oauth_clear_tokens(provider_id)` (per-provider routing lives
    /// in the backend) followed by a `list_provider_credentials` refresh so
    /// the disconnected provider's `oauth-status` (Logout) row disappears.
    /// Backend clear errors are swallowed (status without leaking the RPC
    /// name); the clear is idempotent.
    OAuthDisconnect {
        provider_id: String,
    },

    // ========================================================================
    // PROV-113: OAuth login actions (browser / headless / device). The view
    // emits `OAuthLoginStart` on Enter over a login row; App::dispatch routes
    // by (provider, method) to the backend browser/headless/device calls. The
    // intermediate `OAuthHeadlessReady` / `OAuthDeviceReady` results set the
    // code-entry / device-waiting modes; `OAuthLoginSucceeded` /
    // `OAuthLoginFailed` fold the terminal outcome (dropped when their
    // generation no longer matches the view's, i.e. the flow was cancelled).
    // ========================================================================
    /// PROV-113: start an OAuth login for `provider_id` using `method`. Carries
    /// the view's current generation so a stale result can be dropped.
    OAuthLoginStart {
        provider_id: String,
        method: crate::views::provider_settings::nav_item::OAuthMethod,
        generation: u64,
    },
    /// PROV-113: anthropic headless start resolved — set the code-entry mode.
    OAuthHeadlessReady {
        provider_id: String,
        authorize_url: String,
        pkce_verifier: String,
        generation: u64,
    },
    /// PROV-113: codex device start resolved — set device-waiting + poll.
    OAuthDeviceReady {
        provider_id: String,
        user_code: String,
        verification_url: String,
        device_auth_id: String,
        interval: u64,
        generation: u64,
    },
    /// PROV-113: submit the pasted headless code (anthropic).
    OAuthLoginHeadlessSubmit {
        provider_id: String,
        code: String,
        pkce_verifier: String,
        generation: u64,
    },
    /// PROV-113: a login completed successfully.
    OAuthLoginSucceeded {
        provider_id: String,
        generation: u64,
    },
    /// PROV-113: a login failed (message is UI-safe — no RPC/method name).
    OAuthLoginFailed {
        provider_id: String,
        error: String,
        generation: u64,
    },
    /// PROV-113: open the authorize URL in the user's browser (best-effort).
    OAuthOpenUrl {
        url: String,
    },
    /// PROV-113: copy the authorize URL to the clipboard (best-effort).
    OAuthCopyUrl {
        url: String,
    },
    /// PROV-114: begin the github-copilot device flow. `enterprise_host` is
    /// `None` for GitHub.com or `Some(normalized_host)` for GitHub Enterprise.
    /// Carries the view's current generation so a stale result can be dropped.
    /// App::dispatch spawns `backend.oauth_copilot_device_start(enterprise_host)`
    /// then reuses the PROV-113 `OAuthDeviceReady` → `oauth_device_poll` path.
    OAuthCopilotDeviceStart {
        enterprise_host: Option<String>,
        generation: u64,
    },

    // ========================================================================
    // RPC-056: Blocklist view actions. The `/blocklist` slash command
    // dispatches `OpenBlocklistView`; the App spawns
    // `backend.blocklist_list()` whose result lands as
    // `BlocklistRulesLoaded`. The view emits `ToggleBlocklistRule` on
    // Space/Enter and the App folds the toggle into
    // `AgentViewStore.blocklist_disabled_by_session`. Esc dismisses the
    // view via `CloseBlocklistView`.
    // ========================================================================
    /// RPC-056: emitted by the slash command palette to open the
    /// BlocklistView. App::dispatch flips the Navigator's ViewMode to
    /// Blocklist AND spawns `backend.blocklist_list()`.
    OpenBlocklistView,
    /// RPC-056: emitted by the view on Esc. App::dispatch returns the
    /// Navigator to the Agent view.
    CloseBlocklistView,
    /// RPC-056: a spawned `backend.blocklist_list()` task resolved.
    /// App::dispatch folds the result into the open BlocklistView.
    BlocklistRulesLoaded(Vec<codelet_rpc_types::BlocklistRuleInfo>),
    /// RPC-056: emitted by the view on Space/Enter over a rule. The
    /// App folds the toggle into the focused session's entry in
    /// `AgentViewStore.blocklist_disabled_by_session`.
    ToggleBlocklistRule(String),

    // ========================================================================
    // RPC-057: /merge-worktree flow actions. The slash command spawns
    // `backend.inspect_session_changes(session_id)` and lands on
    // `InspectChangesLoaded`; non-empty results push a
    // `MergeConfirmDialog` onto the compositor. Button activation maps
    // to `MergeConfirmed` / `DiscardConfirmed` / `CancelMergeDialog`.
    // ========================================================================
    /// RPC-057: emitted after a successful
    /// `backend.inspect_session_changes()` round-trip. The App pushes
    /// a `MergeConfirmDialog` seeded with the summary onto the
    /// compositor (or emits a [merge] nothing-to-merge notice when
    /// every count is zero).
    OpenMergeConfirmDialog {
        session_id: codelet_rpc_types::SessionId,
        summary: codelet_rpc_types::SessionChangesSummary,
    },
    /// RPC-057: emitted by the slash command path when
    /// `inspect_session_changes` returns zero changes — kept distinct
    /// from `OpenMergeConfirmDialog` so the App can emit the
    /// nothing-to-merge notice without flashing the dialog.
    InspectChangesLoaded {
        session_id: codelet_rpc_types::SessionId,
        summary: codelet_rpc_types::SessionChangesSummary,
    },
    /// RPC-057: user activated the Merge button. App::dispatch pops
    /// the dialog and spawns
    /// `backend.merge_session_worktree(session_id, FastForward)`.
    MergeConfirmed {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-057: user activated the Discard button. App::dispatch pops
    /// the dialog and spawns
    /// `backend.discard_session_worktree(session_id)`.
    DiscardConfirmed {
        session_id: codelet_rpc_types::SessionId,
    },
    /// RPC-057: user pressed Cancel/Esc. App::dispatch pops the
    /// dialog and fires no backend call.
    CancelMergeDialog,
    /// RPC-058: user submitted a `/schedule …` slash command. The App
    /// catch-all dispatcher routes this to
    /// `dispatch_slash_schedule::handle_schedule_subcommand` which fans out to
    /// the matching backend round-trip + notice formatter.
    ScheduleSubcommandParsed(crate::app::schedule_parser::ScheduleSubcommand),
    /// RPC-059: a `/loop …` submit-line input has been parsed into a
    /// [`crate::app::loop_parser::LoopSubcommand`] enum value. The
    /// catch-all dispatcher routes this to
    /// `dispatch_slash_loop::handle_loop_subcommand` which fans out to the
    /// matching backend round-trip + notice formatter.
    LoopSubcommandParsed(crate::app::loop_parser::LoopSubcommand),

    // ========================================================================
    // RPC-060: CreateSessionDialog + isolated-session creation actions.
    // The `/isolation` slash command and any future "new session" entry
    // point dispatches `OpenCreateSessionDialog`; the dialog itself emits
    // `CreateSessionSubmitted` (on Yes / Yes - Isolated) or
    // `CreateSessionCancelled` (on Cancel / Esc). App::dispatch routes
    // CreateSessionSubmitted into either `backend.create_session(None)`
    // (isolated=false) or `backend.create_isolated_session(None)`
    // (isolated=true), folds the response into Action::SessionCreated,
    // and on error emits `Action::EmitSessionNotice` against the focused
    // session.
    // ========================================================================
    /// RPC-060: emitted by the `/isolation` slash command palette pick
    /// (and any future explicit "new session" trigger). App::dispatch
    /// pushes a fresh [`crate::components::create_session_dialog::CreateSessionDialog`]
    /// onto the Compositor at Priority::Foreground, preselecting the
    /// option indicated by `preselect`.
    OpenCreateSessionDialog {
        preselect: Option<crate::components::create_session_dialog::CreateSessionOption>,
    },
    /// RPC-060: emitted by the CreateSessionDialog on Enter over "Yes"
    /// or "Yes - Isolated". App::dispatch spawns
    /// `backend.create_session(None)` (isolated=false) or
    /// `backend.create_isolated_session(None)` (isolated=true) and
    /// folds the result into `Action::SessionCreated`. On error the
    /// dispatch helper surfaces a `[error] create [isolated ]session: …`
    /// notice to the focused session via `Action::EmitSessionNotice`.
    CreateSessionSubmitted {
        isolated: bool,
    },
    /// RPC-060: emitted by the CreateSessionDialog on Enter over
    /// "Cancel" or on Esc. App::dispatch is a silent no-op — the
    /// callback returned by `handle_event` removes the dialog from the
    /// Compositor.
    CreateSessionCancelled,

    // ========================================================================
    // RPC-061: supervisor / subordinate links surface.
    // SupervisorsLoaded is the result of an async backend.get_supervisors
    // snapshot — the dispatch helper folds the list into AgentViewStore so
    // the SessionHeader paints the cyan `[Subordinate of: <id>]` badge.
    // SendToSubordinate is dispatched by a supervisor's UI to forward an
    // `IncomingMessageInput` payload onto a subordinate session via
    // `backend.receive_incoming_message`.
    // ========================================================================
    /// RPC-061: result of a `backend.get_supervisors(session_id)` snapshot.
    /// `App::dispatch` writes the list into `AgentViewStore` so the
    /// next render of the SessionHeader can paint the subordinate badge.
    SupervisorsLoaded(
        codelet_rpc_types::SessionId,
        Vec<codelet_rpc_types::SessionId>,
    ),
    /// RPC-061: dispatched by a supervisor's UI to forward an
    /// `IncomingMessageInput` payload onto `subordinate_id` via
    /// `backend.receive_incoming_message`. On `Err` the dispatch helper
    /// surfaces a `[error] send to subordinate: <e>` notice against the
    /// originating supervisor session via `Action::EmitSessionNotice`.
    SendToSubordinate {
        subordinate_id: codelet_rpc_types::SessionId,
        message: codelet_rpc_types::IncomingMessageInput,
    },
    /// RPC-079: request the Compositor pop the dialog identified by
    /// the given stable id. Emitted by `NotificationDialog` and
    /// `StatusDialog` (Complete state) when their auto-dismiss timer
    /// fires. `App::dispatch` routes this via `try_dispatch_dialog_dismiss`
    /// which calls `compositor.remove(&id)` and triggers a redraw.
    DismissDialog(String),

    // RPC-373: BoardView `D` opens the project FOUNDATION.md in the user's
    // default browser via the RPC-372 viewer server (no-op when unavailable).
    OpenFoundation,

    // RPC-374: BoardView `A` opens an attachment picker for the selected work
    // unit; `OpenAttachmentPicker` pushes the dialog and `OpenAttachment(path)`
    // launches the browser at the viewer URL (no-op when unavailable).
    OpenAttachmentPicker,
    OpenAttachment(String),

    // RPC-356: Dual-pane ChangedFilesView (BoardView `F` opens; selection →
    // LoadFileDiff → FileDiffLoaded; Esc → close).
    OpenChangedFilesView,
    CloseChangedFilesView,
    ChangedFilesLoaded(Vec<codelet_rpc_types::ChangedFile>),
    LoadFileDiff(String),
    FileDiffLoaded {
        path: String,
        diff: Option<String>,
    },

    // RPC-364: Three-pane CheckpointsView (BoardView `C` opens; selecting a
    // checkpoint → LoadCheckpointFiles → CheckpointFilesLoaded; selecting a
    // file → LoadCheckpointFileDiff → CheckpointFileDiffLoaded; Esc → close).
    OpenCheckpointsView,
    CloseCheckpointsView,
    CheckpointsLoaded(Vec<codelet_rpc_types::CheckpointInfo>),
    LoadCheckpointFiles {
        work_unit_id: String,
        name: String,
    },
    CheckpointFilesLoaded {
        work_unit_id: String,
        name: String,
        files: Vec<codelet_rpc_types::ChangedFile>,
    },
    LoadCheckpointFileDiff {
        work_unit_id: String,
        name: String,
        path: String,
    },
    CheckpointFileDiffLoaded {
        work_unit_id: String,
        name: String,
        path: String,
        diff: Option<String>,
    },

    // RPC-365: Checkpoint restore actions. `r/R` (single file) and `t/T`
    // (all files) open an in-view confirmation dialog; confirming emits a
    // Restore* action which App::dispatch_checkpoints folds into a
    // transport call, then routes a RestoreCheckpointResult back to drive
    // the status dialog (complete | error) and refresh the diff + counts.
    RestoreCheckpointFile {
        work_unit_id: String,
        name: String,
        path: String,
    },
    RestoreCheckpointAll {
        work_unit_id: String,
        name: String,
    },
    RestoreCheckpointResult {
        work_unit_id: String,
        name: String,
        /// `Some(path)` for a single-file restore (drives the diff
        /// reload); `None` for a restore-all.
        path: Option<String>,
        /// `None` on success, `Some(message)` on failure.
        error: Option<String>,
    },
    /// Re-fetch `checkpoint_counts()` so the board header repaints after a
    /// restore changed the working tree.
    RefreshCheckpointCounts,

    // RPC-366: Checkpoint delete actions. `d/D` (single checkpoint) opens a
    // yes/no confirmation; `a/A` (all checkpoints) opens a typed `DELETE
    // ALL` confirmation. Confirming emits a Delete* action which
    // App::dispatch_checkpoint_delete folds into a transport call, then
    // routes a DeleteCheckpointResult back to remove the row / clear the
    // list, clamp the selection (or close the view when empty) and refresh
    // the board counts.
    DeleteCheckpoint {
        work_unit_id: String,
        name: String,
    },
    DeleteAllCheckpoints,
    DeleteCheckpointResult {
        work_unit_id: String,
        name: String,
        /// `true` for a delete-all; `false` for a single-checkpoint delete.
        all: bool,
        /// `None` on success, `Some(message)` on failure.
        error: Option<String>,
    },

    /// RPC-381 SELECT mode (Tab toggle + ↑/↓ turn nav).
    ToggleTurnSelectMode,
    TurnNavUp,
    TurnNavDown,
    /// RPC-382: open/close the turn content modal (Enter on the selected
    /// turn opens it; Esc closes it). Parameterless — the App reducer
    /// resolves the turn `seq` from the focused scrollback's selection.
    OpenTurnModal,
    CloseTurnModal,
    /// RPC-383: scroll the OPEN turn content modal's body (offset clamped;
    /// selection untouched). Up/Down 1 row, Page* a page, Home/End ends.
    TurnModalScrollUp,
    TurnModalScrollDown,
    TurnModalPageUp,
    TurnModalPageDown,
    TurnModalHome,
    TurnModalEnd,

    /// COPY-006: emitted by `views/agent/mouse_dispatch.rs` when a
    /// left-button press lands inside the scrollback rect (drag start /
    /// long-press anchor). App::dispatch begins a live selection on the
    /// focused SessionContext's ScrollbackList at the supplied cell.
    /// Mouse capture stays ON — selection is drawn by the TUI itself.
    SelectionBegin(crate::mouse::selection::Cell),
    /// COPY-010: emitted by a long-press (BeginLine) to select the WHOLE
    /// line under the press. App::dispatch routes it to
    /// `ScrollbackList::selection_begin_line`.
    SelectionBeginLine(crate::mouse::selection::Cell),
    /// COPY-006: emitted as the pointer drags (button-held move) inside
    /// the scrollback rect. App::dispatch extends the live selection's
    /// cursor to the supplied cell and refreshes the highlight spans.
    SelectionExtend(crate::mouse::selection::Cell),
    /// COPY-006: emitted on left-button release (or long-press fire)
    /// when a live selection exists. App::dispatch reconstructs the
    /// selected text and copies it to the terminal clipboard via OSC52.
    /// Per rule [2] the selection is NOT cleared on commit — it stays
    /// highlighted so the user sees what was copied.
    SelectionCommit,
    /// COPY-006: clears any live selection + highlight. Emitted on Esc
    /// (first cascade level) and on any scrollback scroll (rule [7]).
    SelectionClear,
    /// COPY-007: emitted by the AgentView composer on a Commit gesture
    /// carrying the prompt-free selected text. App::dispatch copies it
    /// via OSC 52 (`self.clipboard.copy(text)`), reusing the COPY-006 writer.
    CopyToClipboard(String),
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
