//! The MockBackend fields of one group and their empty defaults.

use super::*;

/// WT-018: how the mock answers `session_card_changes`.
pub(super) type SessionCardChangesFn = Arc<
    dyn Fn(&SessionId) -> std::result::Result<Vec<codelet_rpc_types::SessionCardChange>, String>
        + Send
        + Sync,
>;

/// WT-018: how the mock answers `move_session_cards_back`.
pub(super) type MoveSessionCardsBackFn = Arc<
    dyn Fn(&SessionId) -> std::result::Result<Vec<codelet_rpc_types::CardMoveBack>, String>
        + Send
        + Sync,
>;

pub(super) struct MockOrchestration {
    /// RPC-057: when `Some`, `prune_orphaned_worktrees` returns
    /// `Err(anyhow!(message))`.
    pub(super) prune_orphaned_worktrees_error: Mutex<Option<String>>,
    /// RPC-057: seeded pruned session ids list.
    pub(super) pruned_sessions: Mutex<Vec<String>>,
    /// RPC-057: per-call counter for `list_session_worktrees`.
    pub(super) list_session_worktrees_calls: AtomicUsize,
    /// RPC-057: seeded `SessionWorktreeInfo` list.
    pub(super) session_worktrees: Mutex<Vec<codelet_rpc_types::SessionWorktreeInfo>>,
    /// RPC-057: per-call counter for `inspect_session_changes`.
    pub(super) inspect_session_changes_calls: AtomicUsize,
    /// RPC-057: seeded `SessionChangesSummary`.
    pub(super) session_changes_summary: Mutex<codelet_rpc_types::SessionChangesSummary>,
    /// WT-018: per-call counter for `session_card_changes`.
    pub(super) session_card_changes_calls: AtomicUsize,
    /// WT-018: answers `session_card_changes` (no cards when `None`).
    pub(super) session_card_changes: Mutex<Option<SessionCardChangesFn>>,
    /// WT-018: per-call counter for `move_session_cards_back`.
    pub(super) move_session_cards_back_calls: AtomicUsize,
    /// WT-018: answers `move_session_cards_back` (no cards when `None`).
    pub(super) move_session_cards_back: Mutex<Option<MoveSessionCardsBackFn>>,
    // ── WT-009 detach_session_worktree surface ───────────────────────
    /// WT-009: per-call counter for `detach_session_worktree`.
    pub(super) detach_session_worktree_calls: AtomicUsize,
    /// WT-009: when `Some`, `detach_session_worktree` returns
    /// `Err(anyhow!(message))`.
    pub(super) detach_session_worktree_error: Mutex<Option<String>>,
    // ── RPC-058 /schedule surface ────────────────────────────────────
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_add`. Defaults to `Ok(ScheduledJob::default())`.
    pub(super) schedule_add_result:
        Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<Vec<ScheduledJob>, String>` returned by
    /// `schedule_list`.
    pub(super) schedule_list_result:
        Mutex<std::result::Result<Vec<codelet_rpc_types::ScheduledJob>, String>>,
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_pause`.
    pub(super) schedule_pause_result:
        Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_resume`.
    pub(super) schedule_resume_result:
        Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<(), String>` returned by `schedule_remove`.
    pub(super) schedule_remove_result: Mutex<std::result::Result<(), String>>,
    /// RPC-058: per-call counters.
    pub(super) schedule_add_calls: AtomicUsize,
    pub(super) schedule_list_calls: AtomicUsize,
    pub(super) schedule_pause_calls: AtomicUsize,
    pub(super) schedule_resume_calls: AtomicUsize,
    pub(super) schedule_remove_calls: AtomicUsize,
    // ── RPC-059 /loop surface ────────────────────────────────────────
    /// RPC-059: seeded `Result<RegisteredLoop, String>` returned by
    /// `loop_add`. Defaults to `Ok(RegisteredLoop::default())`.
    pub(super) loop_add_result:
        Mutex<std::result::Result<codelet_rpc_types::RegisteredLoop, String>>,
    /// RPC-059: seeded `Result<bool, String>` returned by `loop_cancel`.
    pub(super) loop_cancel_result: Mutex<std::result::Result<bool, String>>,
    /// RPC-059: seeded `Result<Vec<RegisteredLoop>, String>` returned by
    /// `loop_list`.
    pub(super) loop_list_result:
        Mutex<std::result::Result<Vec<codelet_rpc_types::RegisteredLoop>, String>>,
    /// RPC-059: per-call counters.
    pub(super) loop_add_calls: AtomicUsize,
    pub(super) loop_cancel_calls: AtomicUsize,
    pub(super) loop_list_calls: AtomicUsize,
    /// RPC-060: seeded `Result<IsolatedSessionInfo, String>` returned by
    /// `create_isolated_session`.
    pub(super) create_isolated_session_result:
        Mutex<std::result::Result<IsolatedSessionInfo, String>>,
    /// RPC-060: per-call counter for `create_isolated_session`.
    pub(super) create_isolated_session_calls: AtomicUsize,
    /// RPC-060: the role passed to each `create_isolated_session` call.
    pub(super) created_isolated_session_roles: Mutex<Vec<Option<String>>>,
    /// RPC-061: scripted per-session supervisors list returned by
    /// `get_supervisors`. Indexed by SessionId.
    pub(super) supervisors_results: Mutex<HashMap<SessionId, Vec<SessionId>>>,
    /// TUI-124: scripted per-supervisor subordinates returned by
    /// `get_subordinates` (a destroyed session leaves every list, the
    /// ChainOfCommand cleanup parity).
    pub(super) subordinates_results: Mutex<HashMap<SessionId, Vec<SessionId>>>,
    /// TUI-124: scripted `get_subordinates` failure.
    pub(super) subordinates_error: Mutex<Option<String>>,
    /// TUI-124: scripted per-session `destroy_session` failures.
    pub(super) destroy_session_errors: Mutex<HashMap<SessionId, String>>,
    /// TUI-124: every session `destroy_session` closed, in order.
    pub(super) destroyed_sessions: Mutex<Vec<SessionId>>,
    /// RPC-061: scripted `Result<(), String>` returned by `add_supervisor`.
    /// Default `Ok(())`.
    pub(super) add_supervisor_result: Mutex<std::result::Result<(), String>>,
    /// RPC-061: scripted `Result<(), String>` returned by
    /// `receive_incoming_message`. Default `Ok(())`.
    pub(super) receive_incoming_message_result: Mutex<std::result::Result<(), String>>,
    /// RPC-061: per-call counters for the five new methods.
    pub(super) add_supervisor_calls: AtomicUsize,
    pub(super) remove_supervisor_calls: AtomicUsize,
    pub(super) get_supervisors_calls: AtomicUsize,
    pub(super) get_subordinate_calls: AtomicUsize,
    pub(super) get_subordinates_calls: AtomicUsize,
    pub(super) receive_incoming_message_calls: AtomicUsize,
    /// RPC-061: capture of the last payload passed to
    /// `receive_incoming_message`.
    pub(super) last_received_incoming_message: Mutex<Option<(SessionId, IncomingMessageInput)>>,
    /// RIG-016: every `submit_input` call, in order.
    pub(super) submitted_inputs: Mutex<Vec<(SessionId, String, codelet_rpc_types::InputMode)>>,
    /// BUG-325: scripted failure for the three custom-model writes.
    pub(super) custom_model_write_error: Mutex<Option<String>>,
    /// BUG-328: scripted refusal returned by `send_input`.
    pub(super) send_input_error: Mutex<Option<String>>,
    /// RPC-450: every card details / move request as (kind, workspace, id).
    pub(super) card_requests: Mutex<Vec<(String, String, String)>>,
}

impl Default for MockOrchestration {
    fn default() -> Self {
        Self {
            prune_orphaned_worktrees_error: Mutex::new(None),
            pruned_sessions: Mutex::new(Vec::new()),
            list_session_worktrees_calls: AtomicUsize::new(0),
            session_worktrees: Mutex::new(Vec::new()),
            detach_session_worktree_calls: AtomicUsize::new(0),
            detach_session_worktree_error: Mutex::new(None),
            inspect_session_changes_calls: AtomicUsize::new(0),
            session_changes_summary: Mutex::new(codelet_rpc_types::SessionChangesSummary {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                commits: Vec::new(),
                files_ignored: 0,
            }),
            session_card_changes_calls: AtomicUsize::new(0),
            session_card_changes: Mutex::new(None),
            move_session_cards_back_calls: AtomicUsize::new(0),
            move_session_cards_back: Mutex::new(None),
            schedule_add_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_list_result: Mutex::new(Ok(Vec::new())),
            schedule_pause_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_resume_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_remove_result: Mutex::new(Ok(())),
            schedule_add_calls: AtomicUsize::new(0),
            schedule_list_calls: AtomicUsize::new(0),
            schedule_pause_calls: AtomicUsize::new(0),
            schedule_resume_calls: AtomicUsize::new(0),
            schedule_remove_calls: AtomicUsize::new(0),
            loop_add_result: Mutex::new(Ok(codelet_rpc_types::RegisteredLoop::default())),
            loop_cancel_result: Mutex::new(Ok(true)),
            loop_list_result: Mutex::new(Ok(Vec::new())),
            loop_add_calls: AtomicUsize::new(0),
            loop_cancel_calls: AtomicUsize::new(0),
            loop_list_calls: AtomicUsize::new(0),
            create_isolated_session_result: Mutex::new(Ok(IsolatedSessionInfo {
                session_id: SessionId::new("mock-isolated"),
                worktree_path: String::new(),
                base_commit: String::new(),
            })),
            create_isolated_session_calls: AtomicUsize::new(0),
            created_isolated_session_roles: Mutex::new(Vec::new()),
            supervisors_results: Mutex::new(HashMap::new()),
            subordinates_results: Mutex::new(HashMap::new()),
            subordinates_error: Mutex::new(None),
            destroy_session_errors: Mutex::new(HashMap::new()),
            destroyed_sessions: Mutex::new(Vec::new()),
            add_supervisor_result: Mutex::new(Ok(())),
            receive_incoming_message_result: Mutex::new(Ok(())),
            add_supervisor_calls: AtomicUsize::new(0),
            remove_supervisor_calls: AtomicUsize::new(0),
            get_supervisors_calls: AtomicUsize::new(0),
            get_subordinate_calls: AtomicUsize::new(0),
            get_subordinates_calls: AtomicUsize::new(0),
            receive_incoming_message_calls: AtomicUsize::new(0),
            last_received_incoming_message: Mutex::new(None),
            submitted_inputs: Mutex::new(Vec::new()),
            custom_model_write_error: Mutex::new(None),
            send_input_error: Mutex::new(None),
            card_requests: Mutex::new(Vec::new()),
        }
    }
}
