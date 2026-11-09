//! The MockBackend fields of one group and their empty defaults.

use super::*;

pub(super) struct MockCore {
    /// `list_work_units` result; `Err` scripts a failed listing (BUG-361).
    pub(super) work_units: Mutex<std::result::Result<Vec<WorkUnitInfo>, String>>,
    pub(super) sessions: Mutex<Vec<SessionInfo>>,
    /// BUG-206: per-call counter for `list_sessions` (the disk-merging
    /// /resume list). The auto-restore path must NEVER call it.
    pub(super) list_sessions_calls: AtomicUsize,
    /// RPC-415: wrapped in `Mutex<Option>` (mirroring `chunks_tx`) so the
    /// reconnect-resubscribe tests can drop the Sender via
    /// `disconnect_all()` (subscribers observe `RecvError::Closed`) and
    /// then install a FRESH Sender via `reconnect_all()` — modelling the
    /// transport supervisor swapping in a brand-new RPC client whose
    /// broadcast senders are distinct from the old (dropped) client's.
    pub(super) work_units_tx: Mutex<Option<broadcast::Sender<Vec<WorkUnitInfo>>>>,
    /// RPC-045: wrapped in `Mutex<Option>` so tests can deliberately
    /// drop the Sender via `close_chunks_tx()` to simulate a
    /// SessionManager shutdown and assert the subscriber loop exits
    /// without panicking.
    pub(super) chunks_tx: Mutex<Option<broadcast::Sender<(SessionId, StreamChunk)>>>,
    /// RPC-415: `Mutex<Option>` for the same disconnect/reconnect swap as
    /// `work_units_tx`.
    pub(super) logs_tx: Mutex<Option<broadcast::Sender<LogRecord>>>,
    /// RPC-045: push-driven (SessionId, SessionStatus) broadcast Sender.
    /// Tests use `push_status_change` to drive synthetic transitions
    /// without going through a real SessionManager.
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    pub(super) status_changes_tx: Mutex<Option<broadcast::Sender<(SessionId, SessionStatus)>>>,
    /// RPC-385: push-driven session-created broadcast Sender. Tests use
    /// `push_session_created` to drive synthetic session-creation events
    /// (e.g. an AgentManager-spawned subordinate) without a real
    /// SessionManager. The capacity is intentionally small (matching the
    /// other broadcast channels) so the lag-recovery scenario can overflow
    /// it and force `RecvError::Lagged`.
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    pub(super) session_created_tx: Mutex<Option<broadcast::Sender<SessionInfo>>>,
    /// RPC-434: push-driven session-removed broadcast Sender. Tests use
    /// `push_session_removed` to drive synthetic session-removal events
    /// (a session destroyed from ANOTHER client) without a real
    /// SessionManager. Mirrors `session_created_tx` (RPC-385) exactly —
    /// the payload is the destroyed session's SessionId.
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    pub(super) session_removed_tx: Mutex<Option<broadcast::Sender<SessionId>>>,
    /// RPC-437: push-driven session-cleared broadcast Sender. Tests use
    /// `push_session_cleared` to drive synthetic session-clear events
    /// (a session cleared from ANOTHER client, or the NAPI/Telegram
    /// entry point) without a real SessionManager. Mirrors
    /// `session_removed_tx` (RPC-434) exactly — the payload is the
    /// cleared session's SessionId. Unlike removal, the session stays
    /// LIVE (clear is repeatable).
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    pub(super) session_cleared_tx: Mutex<Option<broadcast::Sender<SessionId>>>,
    /// RPC-434: scripted connect-time open-session snapshot — the mock
    /// equivalent of the server-pushed `Envelope::SessionSnapshot`
    /// frame (WS) / `SharedFspecService::live_sessions_snapshot`
    /// (embedded). `seed_session_snapshot` primes it; the
    /// `session_snapshot` trait method drains it.
    pub(super) scripted_session_snapshot: Mutex<Vec<SessionInfo>>,
    /// TUI-109: push-driven checkpoint-enumeration progress broadcast
    /// Sender. Tests use `push_checkpoints_progress` to drive synthetic
    /// `CheckpointsProgress` frames without a real RPC server.
    pub(super) checkpoints_progress_tx:
        Mutex<Option<broadcast::Sender<codelet_rpc_types::CheckpointsProgress>>>,
    /// BUG-181: push-driven checkpoint-changed broadcast Sender. Tests
    /// use `push_checkpoint_counts_changed` to drive synthetic
    /// `CheckpointCounts` frames (mirrors the
    /// `push_checkpoints_progress` pattern from TUI-109).
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    pub(super) git_state_changed_tx: Mutex<Option<broadcast::Sender<GitState>>>,
    /// BUG-182: per-call counters for the lazy-view RPCs the mux entry
    /// flow + git-state refresh drive.
    pub(super) changed_files_calls: AtomicUsize,
    pub(super) list_checkpoints_calls: AtomicUsize,
    /// BUG-184: scripted payloads for the lazy-view RPCs so App-level
    /// tests can drive a LOADED Files/Checkpoints pane (the trait
    /// defaults return empty Vecs).
    pub(super) scripted_changed_files: Mutex<Vec<codelet_rpc_types::ChangedFile>>,
    pub(super) scripted_checkpoints: Mutex<Vec<codelet_rpc_types::CheckpointInfo>>,
    pub(super) scripted_checkpoint_files:
        Mutex<Option<(String, String, Vec<codelet_rpc_types::ChangedFile>)>>,
    pub(super) scripted_checkpoint_file_diffs: Mutex<Option<ScriptedCheckpointFileDiff>>,
    pub(super) list_work_units_calls: AtomicUsize,
    pub(super) create_session_calls: AtomicUsize,
    pub(super) created_session_roles: Mutex<Vec<Option<String>>>,
    pub(super) send_input_calls: AtomicUsize,
    pub(super) interrupt_calls: AtomicUsize,
    pub(super) checkpoint_counts_calls: AtomicUsize,
    /// RPC-365: per-call counters + last-path capture for the restore
    /// transport methods so App-dispatch tests can assert the wiring.
    pub(super) restore_checkpoint_file_calls: AtomicUsize,
    pub(super) restore_checkpoint_all_calls: AtomicUsize,
    pub(super) last_restore_file: Mutex<Option<(String, String, String)>>,
    /// RPC-366: per-call counters + last-key capture for the delete
    /// transport methods so App-dispatch tests can assert the wiring.
    pub(super) delete_checkpoint_calls: AtomicUsize,
    pub(super) delete_all_checkpoints_calls: AtomicUsize,
    pub(super) last_delete_checkpoint: Mutex<Option<(String, String)>>,
    /// RPC-017: per-call counters for the reorder methods + record of
    /// the most recently passed id so App-level dispatch tests can
    /// assert that `Action::ReorderUp`/`Down` routes to the focused-
    /// column selection.
    pub(super) move_work_unit_up_calls: AtomicUsize,
    pub(super) move_work_unit_down_calls: AtomicUsize,
    pub(super) last_move_work_unit_up_id: Mutex<Option<String>>,
    pub(super) last_move_work_unit_down_id: Mutex<Option<String>>,
    pub(super) scripted_session: Mutex<Option<std::result::Result<SessionId, String>>>,
    pub(super) last_send_input: Mutex<Option<(SessionId, String)>>,
    pub(super) last_interrupt: Mutex<Option<SessionId>>,
    /// RPC-098: counter incremented every time `destroy_session` is called
    /// via the FspecBackend trait. Lets ESC exit-confirmation tests assert
    /// that Close reaches the backend and Cancel does NOT.
    pub(super) destroy_session_calls: AtomicUsize,
    /// RPC-098: most recently destroyed SessionId — paired counterpart to
    /// `destroy_session_calls`.
    pub(super) last_destroyed_session: Mutex<Option<SessionId>>,
    pub(super) checkpoint_counts: Mutex<std::result::Result<CheckpointCounts, String>>,
    /// RPC-018: scripted ModelInfo returned by `get_model_info`.
    pub(super) model_info: Mutex<ModelInfo>,
    /// RPC-018: scripted ThinkingLevel returned by `get_thinking_level`.
    pub(super) thinking_level: Mutex<ThinkingLevel>,
    /// MODEL-011: scripted options returned by `get_thinking_options`;
    /// `None` makes the call fail.
    pub(super) thinking_options: Mutex<Option<codelet_rpc_types::ThinkingOptions>>,
    /// MODEL-011: `set_thinking_level_default` calls and the last level.
    pub(super) set_thinking_level_default_calls: AtomicUsize,
    pub(super) last_set_thinking_level_default: Mutex<Option<ThinkingLevel>>,
    /// RPC-018: scripted WorkspaceInfo returned by `get_workspace_info`.
    pub(super) workspace_info: Mutex<WorkspaceInfo>,
    /// RPC-018: when `Some`, `get_workspace_info` returns
    /// `Err(anyhow!(message))` so bootstrap-best-effort scenarios can
    /// exercise the failure branch.
    pub(super) workspace_info_error: Mutex<Option<String>>,
    /// RPC-020: scripted file-search results returned by
    /// `search_files`. The mock filters by case-insensitive substring
    /// of `prefix` and caps at `limit`.
    pub(super) file_search_results: Mutex<Vec<String>>,
    /// RPC-026: per-call counter for `persistence_delete_session`.
    pub(super) delete_session_calls: AtomicUsize,
    /// RPC-026: capture of the last id passed to
    /// `persistence_delete_session`.
    pub(super) last_deleted_session: Mutex<Option<SessionId>>,
    /// RPC-427: capture of the last project_path passed to
    /// `list_sessions`. Tests use `list_sessions_project()` to
    /// assert that the TUI passes the current working directory.
    pub(super) last_list_sessions_project: Mutex<Option<String>>,
    /// RPC-026: scripted history-search results returned by
    /// `persistence_search_history`. Indexed by query string.
    pub(super) history_search_results: Mutex<Vec<codelet_rpc_types::HistoryMatch>>,
    /// RPC-026: per-call counter for `persistence_search_history`.
    pub(super) search_history_calls: AtomicUsize,
    /// RPC-026: capture of the last query passed to
    /// `persistence_search_history`.
    pub(super) last_history_query: Mutex<Option<String>>,
    /// RPC-022: scripted providers returned by `list_providers`.
    pub(super) providers: Mutex<Vec<ProviderInfo>>,
    /// RPC-451: a scripted provider-listing failure.
    pub(super) list_providers_error: Mutex<Option<String>>,
    /// BUG-207: method names recorded in await order (currently
    /// `list_providers` and `refresh_models_cache`) so tests can assert
    /// cross-method ordering, not just per-method counts.
    pub(super) call_log: Mutex<Vec<String>>,
    /// RPC-022: counters + captures for the five new RPC methods.
    pub(super) list_providers_calls: AtomicUsize,
    pub(super) set_session_model_calls: AtomicUsize,
    pub(super) set_thinking_level_calls: AtomicUsize,
    pub(super) get_session_role_calls: AtomicUsize,
    pub(super) set_session_role_calls: AtomicUsize,
    pub(super) last_set_session_model: Mutex<Option<(SessionId, String, String)>>,
    /// PROV-118: per-call counter + capture for `set_default_model`.
    pub(super) set_default_model_calls: AtomicUsize,
    pub(super) last_set_default_model: Mutex<Option<String>>,
    pub(super) last_set_thinking_level: Mutex<Option<(SessionId, ThinkingLevel)>>,
    pub(super) last_get_session_role: Mutex<Option<SessionId>>,
    pub(super) last_set_session_role: Mutex<Option<(SessionId, Option<String>)>>,
    /// RPC-022: scripted per-session role overlay returned by
    /// `get_session_role` and overwritten by `set_session_role`. Mutex
    /// of a Vec because tests sometimes seed multiple sessions.
    pub(super) session_roles: Mutex<Vec<(SessionId, Option<String>)>>,
    /// RPC-022: per-call counter for `persistence_add_history`.
    pub(super) persistence_add_history_calls: AtomicUsize,
    /// RPC-022: capture of the last `(SessionId, text)` passed to
    /// `persistence_add_history`.
    pub(super) last_persistence_add_history: Mutex<Option<(SessionId, String)>>,
    /// RPC-045: per-call counter for `send_fspec_result`.
    pub(super) send_fspec_result_calls: AtomicUsize,
    /// RPC-045: capture of the last `FspecResult` passed to
    /// `send_fspec_result`. The matching `SessionId` is co-stored so
    /// tests assert both pieces of the round-trip.
    pub(super) last_fspec_result: Mutex<Option<(SessionId, FspecResult)>>,
    /// RPC-046: per-call counter for `clear_history`.
    pub(super) clear_history_calls: AtomicUsize,
    /// RPC-046: capture of the last SessionId passed to
    /// `clear_history`.
    pub(super) last_clear_history_session: Mutex<Option<SessionId>>,
    /// RPC-046: when `Some`, `clear_history` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch.
    pub(super) clear_history_error: Mutex<Option<String>>,
    /// RPC-047: per-call counter for `compact_session`.
    pub(super) compact_session_calls: AtomicUsize,
    /// RPC-047: capture of the last SessionId passed to
    /// `compact_session`.
    pub(super) last_compact_session: Mutex<Option<SessionId>>,
    /// RPC-047: scripted result returned by `compact_session`. Defaults
    /// to a 1.0 compression-ratio Ok response so scenarios that don't
    /// care about the body still get a well-formed reply.
    pub(super) compact_session_result: Mutex<Result<CompactionResult, String>>,
    /// RPC-049: per-call counter for `resume_session`.
    pub(super) resume_session_calls: AtomicUsize,
    /// RPC-049: capture of the last SessionId passed to `resume_session`.
    pub(super) last_resume_session: Mutex<Option<SessionId>>,
    /// RPC-049: when `Some`, `resume_session` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch.
    pub(super) resume_session_error: Mutex<Option<String>>,
    /// RPC-049: scripted replay-buffer returned by `get_buffered_output`.
    pub(super) buffered_output: Mutex<Vec<StreamChunk>>,
    pub(super) transcript_requests: Mutex<Vec<SessionId>>,
    /// RPC-049: per-call counter for `get_buffered_output`.
    pub(super) get_buffered_output_calls: AtomicUsize,
    /// RPC-049: capture of the last (id, limit) pair passed to
    /// `get_buffered_output`.
    pub(super) last_get_buffered_output: Mutex<Option<(SessionId, u32)>>,
    /// TUI-113: scripted active session returned by `get_active_session`
    /// (RPC-037 default returns `Ok(None)` — the mock overrides so
    /// bootstrap/reconnect restore tests can script a daemon pointer).
    pub(super) scripted_active_session: Mutex<Option<SessionId>>,
    /// BUG-206: scripted live-session IDs returned by
    /// `list_live_sessions` (the daemon's in-memory session map). The
    /// auto-restore liveness gate — a restore candidate must appear
    /// here to be restored; closed/manifest-only sessions must NOT.
    pub(super) scripted_live_sessions: Mutex<Vec<SessionId>>,
    /// BUG-206: per-call counter for `list_live_sessions`.
    pub(super) list_live_sessions_calls: AtomicUsize,
    /// TUI-113: per-call counter for `set_active_session`.
    pub(super) set_active_session_calls: AtomicUsize,
    /// TUI-113: capture of the last SessionId passed to
    /// `set_active_session` (R3 pointer-tracking assertions).
    pub(super) last_set_active_session: Mutex<Option<SessionId>>,
    /// RPC-050: per-call counter for `set_work_unit_context`.
    pub(super) set_work_unit_context_calls: AtomicUsize,
    /// RPC-050: per-call counter for `get_work_unit_context`.
    pub(super) get_work_unit_context_calls: AtomicUsize,
    /// RPC-050: capture of the last `(SessionId, Option<WorkUnitContext>)`
    /// passed to `set_work_unit_context`.
    pub(super) last_set_work_unit_context: Mutex<Option<(SessionId, Option<WorkUnitContext>)>>,
    /// RPC-050: in-memory work-unit context store so `get_work_unit_context`
    /// reads back what `set_work_unit_context` wrote. Mirrors the
    /// production StubSessionManagerHandle's `work_unit_ctx` map.
    pub(super) work_unit_contexts: Mutex<HashMap<SessionId, WorkUnitContext>>,
    pub(super) focus_state: focus_backend::FocusState,
    /// RPC-050: when `Some`, `set_work_unit_context` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch without touching the in-memory store.
    pub(super) set_work_unit_context_error: Mutex<Option<String>>,
}
