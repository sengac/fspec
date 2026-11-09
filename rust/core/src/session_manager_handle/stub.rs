//! StubSessionManagerHandle — minimal in-memory implementation used by
//! tests (REFAC-022: moved out of session_manager_handle.rs).

use super::*;

// ============================================================================
// StubSessionManagerHandle — minimal in-memory implementation used by tests
// ============================================================================

/// Minimal in-memory implementation used by integration tests.
///
/// Holds an internal session table and emits a deterministic
/// `[StreamChunk::Text("hi back"), StreamChunk::Done]` sequence on
/// `send_input` regardless of input. Replaces the heavy
/// `SessionManager` for cross-transport tests so the tests don't
/// depend on the full provider/tool dependency tree.
pub struct StubSessionManagerHandle {
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
    /// RPC-433: session-lifecycle event senders so multi-client fan-out
    /// tests can drive created/removed events through the real
    /// `SharedFspecService` → WS-server path. `create_session` /
    /// `create_isolated_session` fire `session_created_tx`;
    /// `destroy_session` fires `session_removed_tx` only when the
    /// session actually existed (mirrors the real `SessionManager`).
    session_created_tx: broadcast::Sender<SessionInfo>,
    session_removed_tx: broadcast::Sender<SessionId>,
    /// RPC-436: session-cleared event sender. `clear_history` fires
    /// `session_cleared_tx` only when the session exists in `sessions`
    /// (mirrors the `destroy_session` liveness gate).
    session_cleared_tx: broadcast::Sender<SessionId>,
    sessions: Arc<Mutex<Vec<SessionRecord>>>,
    next_id: AtomicU64,
    next_iso_id: AtomicU64,
    providers: Arc<Mutex<Vec<ProviderInfo>>>,
    // RPC-037 deterministic state seeds — keyed by SessionId.
    tokens: Arc<Mutex<HashMap<SessionId, SessionTokens>>>,
    models: Arc<Mutex<HashMap<SessionId, SessionModel>>>,
    work_unit_ctx: Arc<Mutex<HashMap<SessionId, WorkUnitContext>>>,
    /// BUG-411: the stub's default model, so set/get round-trip.
    default_model: Arc<Mutex<Option<String>>>,
    pending_input: Arc<Mutex<HashMap<SessionId, String>>>,
    debug_enabled: Arc<Mutex<HashMap<SessionId, bool>>>,
    pause_state: Arc<Mutex<HashMap<SessionId, PendingRequest<PauseState>>>>,
    hitl_request: Arc<Mutex<HashMap<SessionId, PendingRequest<HitlRequest>>>>,
    active_session: Arc<Mutex<Option<SessionId>>>,
    /// BUG-541: working directories given with `set_effective_cwd`.
    effective_cwds: Arc<Mutex<HashMap<SessionId, PathBuf>>>,
    // RPC-049: per-stub counter of `resume_session` calls. Used by
    // cross-transport parity tests to assert the round-trip lands on
    // the underlying handle once per transport.
    resume_session_calls: AtomicU64,
    // RPC-050: per-stub counters for the work-unit context RPCs.
    // Used by cross-transport parity tests to assert each transport
    // lands one call on the underlying handle.
    set_work_unit_context_calls: AtomicU64,
    get_work_unit_context_calls: AtomicU64,
    // RPC-054: per-stub state + counters for the provider credentials
    // surface. The map is keyed by provider_id and persists across
    // get/set/delete calls so cross-transport parity tests can assert
    // both transports observe the same in-memory state.
    provider_credentials: Arc<Mutex<HashMap<String, ProviderCredentialInfo>>>,
    set_provider_credentials_calls: AtomicU64,
    delete_provider_credentials_calls: AtomicU64,
    test_provider_connection_calls: AtomicU64,
    refresh_models_cache_calls: AtomicU64,
    list_provider_credentials_calls: AtomicU64,
    // RPC-055: per-stub counters for the debug-capture surface.
    // Cross-transport parity tests use these to assert each transport
    // lands one call on the underlying handle.
    toggle_debug_calls: AtomicU64,
    set_debug_directory_calls: AtomicU64,
    // RPC-056: per-stub seeded blocklist rules + per-call counter for
    // the `blocklist_list` RPC method. Cross-transport parity tests
    // assert each transport lands one call here and returns the same
    // seeded payload.
    blocklist_rules: Arc<Mutex<Vec<BlocklistRuleInfo>>>,
    blocklist_list_calls: AtomicU64,
    /// BUG-601: each session's turned-off blocklist rule ids.
    blocklist_disabled: Arc<Mutex<HashMap<SessionId, std::collections::BTreeSet<String>>>>,
    // RPC-057: per-stub seeded payloads + per-call counters for the
    // merge/discard/prune/list/inspect worktree RPC methods.
    merge_outcome: Arc<Mutex<MergeOutcome>>,
    pruned_sessions: Arc<Mutex<Vec<String>>>,
    session_worktrees: Arc<Mutex<Vec<SessionWorktreeInfo>>>,
    session_changes_summary: Arc<Mutex<SessionChangesSummary>>,
    merge_session_worktree_calls: AtomicU64,
    discard_session_worktree_calls: AtomicU64,
    prune_orphaned_worktrees_calls: AtomicU64,
    list_session_worktrees_calls: AtomicU64,
    inspect_session_changes_calls: AtomicU64,
    // WT-009: per-call counter for the `detach_session_worktree` RPC.
    detach_session_worktree_calls: AtomicU64,
    // RPC-058: per-stub seeded payloads + per-call counters for the
    // /schedule RPC surface (add / list / pause / resume / remove).
    scheduled_jobs: Arc<Mutex<Vec<ScheduledJob>>>,
    schedule_add_calls: AtomicU64,
    schedule_list_calls: AtomicU64,
    schedule_pause_calls: AtomicU64,
    schedule_resume_calls: AtomicU64,
    schedule_remove_calls: AtomicU64,
    // RPC-059: per-stub seeded payloads + per-call counters for the
    // /loop RPC surface (add / cancel / list).
    registered_loops: Arc<Mutex<Vec<RegisteredLoop>>>,
    loop_cancel_result: std::sync::atomic::AtomicBool,
    loop_add_calls: AtomicU64,
    loop_cancel_calls: AtomicU64,
    loop_list_calls: AtomicU64,
    // RPC-061: ChainOfCommand-equivalent maps + per-method call
    // counters + recorded IncomingMessageInput payloads for the
    // supervisor / subordinate surface. add_supervisor implements
    // BFS cycle detection that mirrors `codelet_sessions::ChainOfCommand`.
    sub_to_sup: Arc<Mutex<HashMap<SessionId, Vec<SessionId>>>>,
    sup_to_subs: Arc<Mutex<HashMap<SessionId, Vec<SessionId>>>>,
    recorded_incoming_messages: Arc<Mutex<Vec<(SessionId, IncomingMessageInput)>>>,
    add_supervisor_calls: AtomicU64,
    remove_supervisor_calls: AtomicU64,
    get_supervisors_calls: AtomicU64,
    get_subordinate_calls: AtomicU64,
    get_subordinates_calls: AtomicU64,
    receive_incoming_message_calls: AtomicU64,
    // RPC-347: in-memory custom-model write surface keyed by profile name.
    // The three trait overrides mirror profile_sections append/replace/delete
    // semantics so cross-transport parity tests can assert state without disk.
    custom_models: Arc<Mutex<HashMap<String, Vec<CustomModelDefinition>>>>,
    // BUG-520: while a test holds turns (`hold_turns`), a turn started by
    // `send_input` waits here between its Text and its Done.
    turn_hold: Arc<Mutex<Option<tokio::sync::watch::Receiver<bool>>>>,
}

/// BUG-520: a hold on the stub's turns, from
/// [`StubSessionManagerHandle::hold_turns`]. Turns started while it is held
/// emit their text, then wait for [`StubTurnGate::release`] before emitting
/// `Done` and returning the session to Idle. Dropping the gate releases the
/// turns too, so a held turn can never outlive its test.
#[derive(Debug)]
pub struct StubTurnGate {
    released: tokio::sync::watch::Sender<bool>,
}

impl StubTurnGate {
    /// Let every turn held by this gate finish.
    pub fn release(self) {
        let _ = self.released.send(true);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SessionRecord {
    id: SessionId,
    role: Option<String>,
    status: SessionStatus,
    /// RPC-037: track whether this session was minted by
    /// `create_isolated_session` so `list_sessions` can faithfully
    /// report `is_isolated: true` for those rows. Defaults to false
    /// for plain `create_session` rows.
    is_isolated: bool,
    /// RPC-037: matching worktree path string for isolated rows.
    /// Empty / None for non-isolated sessions.
    worktree_path: Option<String>,
}

impl Default for StubSessionManagerHandle {
    fn default() -> Self {
        Self::new()
    }
}

mod stub_impl_1;
mod stub_impl_2;

include!("stub/stub_handle_1.rs");
include!("stub/stub_handle_2.rs");
include!("stub/stub_handle_3.rs");
include!("stub/stub_handle_4.rs");

macro_rules! compose_stub_handle_finish {
    ($($collected:tt)*) => {
impl SessionManagerHandle for StubSessionManagerHandle {
    $($collected)*
}
    };
}

compose_stub_handle_1! {}

/// BUG-412: lock a stub table. A poisoned lock (a test thread panicked while
/// holding it) is logged and its data is used as-is, so the stub never
/// reports an empty or default table in place of what it actually holds.
fn stub_lock<'a, T>(mutex: &'a Mutex<T>, table: &str) -> std::sync::MutexGuard<'a, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        tracing::error!(
            table,
            "StubSessionManagerHandle lock poisoned; using its data as-is"
        );
        poisoned.into_inner()
    })
}

#[cfg(test)]
mod bug412_tests {
    //! Feature: spec/features/stub-session-manager-handle-lock-poisoning.feature
    use super::stub_lock;
    use std::sync::{Arc, Mutex};

    #[test]
    // Poisoning a Mutex requires a thread to panic while holding it.
    #[allow(clippy::panic)]
    fn a_poisoned_stub_table_is_read_with_its_data() -> Result<(), String> {
        // @step Given a stub table holding one entry
        let table = Arc::new(Mutex::new(vec!["stub-1".to_string()]));
        // @step And a thread panicked while holding the table's lock
        let held = Arc::clone(&table);
        let outcome = std::thread::spawn(move || {
            let _guard = held.lock();
            std::panic::panic_any("test thread panics holding the stub table");
        })
        .join();
        if outcome.is_ok() || !table.is_poisoned() {
            return Err("the table lock must be poisoned for this scenario".into());
        }
        // @step When the stub locks the table
        let entries = stub_lock(&table, "sessions").clone();
        // @step Then the table still holds its one entry
        assert_eq!(entries, vec!["stub-1".to_string()]);
        Ok(())
    }

    #[test]
    fn no_stub_table_read_swallows_a_poisoned_lock() -> Result<(), String> {
        // @step Given the StubSessionManagerHandle source
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/session_manager_handle/stub");
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // @step When its lock acquisitions are listed
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let compact: String = source.split_whitespace().collect();
            // @step Then each one goes through stub_lock or returns the poisoning as an error
            for (index, _) in compact.match_indices(".lock()") {
                let after = &compact[index + ".lock()".len()..];
                assert!(
                    after.starts_with(".map_err("),
                    "{} reads a lock without stub_lock or an error: {}",
                    path.display(),
                    &compact[index..(index + 60).min(compact.len())]
                );
            }
        }
        Ok(())
    }
}
