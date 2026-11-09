//! StubSessionManagerHandle inherent helpers (part 1).

use super::*;

impl StubSessionManagerHandle {
    /// Construct a new stub backed by a deterministic ` [Text, Done]`
    /// emission policy.
    pub fn new() -> Self {
        Self::with_capacity(256, 1024)
    }
    /// Construct a stub matching what an external StubProvider would do —
    /// parameter is ignored; kept for API parity with the test harness.
    pub fn with_provider<P>(_provider: Arc<P>) -> Self {
        Self::new()
    }
    /// Build the wire [`SessionInfo`] for a record (same field mapping as
    /// `list_sessions`). RPC-433: used by the session-lifecycle event
    /// fires and by `live_session_infos`.
    pub(super) fn record_info(r: &SessionRecord) -> SessionInfo {
        SessionInfo {
            id: r.id.value.clone(),
            name: r.id.value.clone(),
            status: status_str(r.status).to_string(),
            project: String::new(),
            message_count: 0,
            provider_id: None,
            model_id: None,
            is_isolated: r.is_isolated,
            worktree_path: r.worktree_path.clone(),
            role: r.role.clone(),
            updated_at_ms: None,
            work_unit_focus: WorkUnitFocus::default(),
        }
    }
    /// Construct a stub with custom broadcast capacities (mostly useful
    /// for stress tests that need bigger buffers).
    pub fn with_capacity(chunks_capacity: usize, logs_capacity: usize) -> Self {
        let (chunks_tx, _) = broadcast::channel(chunks_capacity);
        let (logs_tx, _) = broadcast::channel(logs_capacity);
        let (status_changes_tx, _) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        // RPC-433: session-lifecycle events use the same capacity as
        // status changes; both are low-rate control-plane events.
        let (session_created_tx, _) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        let (session_removed_tx, _) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        // RPC-436: cleared events share the same low-rate capacity.
        let (session_cleared_tx, _) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        Self {
            chunks_tx,
            logs_tx,
            status_changes_tx,
            session_created_tx,
            session_removed_tx,
            session_cleared_tx,
            sessions: Arc::new(Mutex::new(Vec::new())),
            next_id: AtomicU64::new(1),
            next_iso_id: AtomicU64::new(1),
            providers: Arc::new(Mutex::new(Vec::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            models: Arc::new(Mutex::new(HashMap::new())),
            work_unit_ctx: Arc::new(Mutex::new(HashMap::new())),
            default_model: Arc::new(Mutex::new(None)),
            pending_input: Arc::new(Mutex::new(HashMap::new())),
            debug_enabled: Arc::new(Mutex::new(HashMap::new())),
            pause_state: Arc::new(Mutex::new(HashMap::new())),
            hitl_request: Arc::new(Mutex::new(HashMap::new())),
            active_session: Arc::new(Mutex::new(None)),
            effective_cwds: Arc::new(Mutex::new(HashMap::new())),
            turn_hold: Arc::new(Mutex::new(None)),
            resume_session_calls: AtomicU64::new(0),
            set_work_unit_context_calls: AtomicU64::new(0),
            get_work_unit_context_calls: AtomicU64::new(0),
            provider_credentials: Arc::new(Mutex::new(HashMap::new())),
            set_provider_credentials_calls: AtomicU64::new(0),
            delete_provider_credentials_calls: AtomicU64::new(0),
            test_provider_connection_calls: AtomicU64::new(0),
            refresh_models_cache_calls: AtomicU64::new(0),
            list_provider_credentials_calls: AtomicU64::new(0),
            toggle_debug_calls: AtomicU64::new(0),
            set_debug_directory_calls: AtomicU64::new(0),
            blocklist_rules: Arc::new(Mutex::new(Vec::new())),
            blocklist_list_calls: AtomicU64::new(0),
            blocklist_disabled: Arc::new(Mutex::new(HashMap::new())),
            merge_outcome: Arc::new(Mutex::new(MergeOutcome {
                status: MergeStatus::NoChanges,
                conflicts: Vec::new(),
                merge_commit: None,
                worktree_path: None,
            })),
            pruned_sessions: Arc::new(Mutex::new(Vec::new())),
            session_worktrees: Arc::new(Mutex::new(Vec::new())),
            session_changes_summary: Arc::new(Mutex::new(SessionChangesSummary {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                commits: Vec::new(),
                files_ignored: 0,
            })),
            merge_session_worktree_calls: AtomicU64::new(0),
            discard_session_worktree_calls: AtomicU64::new(0),
            prune_orphaned_worktrees_calls: AtomicU64::new(0),
            list_session_worktrees_calls: AtomicU64::new(0),
            inspect_session_changes_calls: AtomicU64::new(0),
            detach_session_worktree_calls: AtomicU64::new(0),
            scheduled_jobs: Arc::new(Mutex::new(Vec::new())),
            schedule_add_calls: AtomicU64::new(0),
            schedule_list_calls: AtomicU64::new(0),
            schedule_pause_calls: AtomicU64::new(0),
            schedule_resume_calls: AtomicU64::new(0),
            schedule_remove_calls: AtomicU64::new(0),
            registered_loops: Arc::new(Mutex::new(Vec::new())),
            loop_cancel_result: std::sync::atomic::AtomicBool::new(true),
            loop_add_calls: AtomicU64::new(0),
            loop_cancel_calls: AtomicU64::new(0),
            loop_list_calls: AtomicU64::new(0),
            // RPC-061: supervisor / subordinate surface state.
            sub_to_sup: Arc::new(Mutex::new(HashMap::new())),
            sup_to_subs: Arc::new(Mutex::new(HashMap::new())),
            recorded_incoming_messages: Arc::new(Mutex::new(Vec::new())),
            add_supervisor_calls: AtomicU64::new(0),
            remove_supervisor_calls: AtomicU64::new(0),
            get_supervisors_calls: AtomicU64::new(0),
            get_subordinate_calls: AtomicU64::new(0),
            get_subordinates_calls: AtomicU64::new(0),
            receive_incoming_message_calls: AtomicU64::new(0),
            custom_models: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    /// RPC-347: snapshot the in-memory custom-model list for `profile_name`
    /// (empty when the profile has none). Cross-transport parity tests use
    /// this to assert both transports forwarded identical definitions.
    pub fn custom_models(&self, profile_name: &str) -> Vec<CustomModelDefinition> {
        stub_lock(&self.custom_models, "custom_models")
            .get(profile_name)
            .cloned()
            .unwrap_or_default()
    }
    /// RPC-049: how many times `resume_session` has been called on this
    /// stub. Used by cross-transport parity tests to assert the
    /// round-trip lands on the underlying handle once per transport.
    pub fn resume_session_calls(&self) -> u64 {
        self.resume_session_calls.load(Ordering::SeqCst)
    }
    /// RPC-050: how many times `set_work_unit_context` has been called
    /// on this stub. Used by cross-transport parity tests.
    pub fn set_work_unit_context_calls(&self) -> u64 {
        self.set_work_unit_context_calls.load(Ordering::SeqCst)
    }
    /// RPC-050: how many times `get_work_unit_context` has been called
    /// on this stub. Used by cross-transport parity tests.
    pub fn get_work_unit_context_calls(&self) -> u64 {
        self.get_work_unit_context_calls.load(Ordering::SeqCst)
    }
    /// RPC-054: how many times `set_provider_credentials` has been
    /// called on this stub. Drives cross-transport parity tests.
    pub fn set_provider_credentials_calls(&self) -> u64 {
        self.set_provider_credentials_calls.load(Ordering::SeqCst)
    }
    /// RPC-054: how many times `delete_provider_credentials` has been
    /// called on this stub.
    pub fn delete_provider_credentials_calls(&self) -> u64 {
        self.delete_provider_credentials_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-054: how many times `test_provider_connection` has been
    /// called on this stub.
    pub fn test_provider_connection_calls(&self) -> u64 {
        self.test_provider_connection_calls.load(Ordering::SeqCst)
    }
    /// RPC-054: how many times `refresh_models_cache` has been called
    /// on this stub.
    pub fn refresh_models_cache_calls(&self) -> u64 {
        self.refresh_models_cache_calls.load(Ordering::SeqCst)
    }
    /// RPC-054: how many times `list_provider_credentials` has been
    /// called on this stub.
    pub fn list_provider_credentials_calls(&self) -> u64 {
        self.list_provider_credentials_calls.load(Ordering::SeqCst)
    }
    /// RPC-055: how many times `toggle_debug` has been called on this
    /// stub. Used by cross-transport parity tests to assert each
    /// transport lands one call on the underlying handle.
    pub fn toggle_debug_calls(&self) -> u64 {
        self.toggle_debug_calls.load(Ordering::SeqCst)
    }
    /// RPC-055: how many times `set_debug_directory` has been called on
    /// this stub.
    pub fn set_debug_directory_calls(&self) -> u64 {
        self.set_debug_directory_calls.load(Ordering::SeqCst)
    }
    /// RPC-056: how many times `blocklist_list` has been called on this
    /// stub. Used by cross-transport parity tests to assert each
    /// transport lands one call on the underlying handle.
    pub fn blocklist_list_calls(&self) -> u64 {
        self.blocklist_list_calls.load(Ordering::SeqCst)
    }
    /// RPC-056: replace the in-memory blocklist rule list returned by
    /// `blocklist_list`. Used by the cross-transport parity test to
    /// seed deterministic data both transports can read identically.
    pub fn seed_blocklist_rules(&self, rules: Vec<BlocklistRuleInfo>) {
        {
            let mut g = stub_lock(&self.blocklist_rules, "blocklist_rules");
            *g = rules;
        }
    }
    /// RPC-057: per-call counter — `merge_session_worktree`.
    pub fn merge_session_worktree_calls(&self) -> u64 {
        self.merge_session_worktree_calls.load(Ordering::SeqCst)
    }
    /// RPC-057: per-call counter — `discard_session_worktree`.
    pub fn discard_session_worktree_calls(&self) -> u64 {
        self.discard_session_worktree_calls.load(Ordering::SeqCst)
    }
    /// RPC-057: per-call counter — `prune_orphaned_worktrees`.
    pub fn prune_orphaned_worktrees_calls(&self) -> u64 {
        self.prune_orphaned_worktrees_calls.load(Ordering::SeqCst)
    }
    /// RPC-057: per-call counter — `list_session_worktrees`.
    pub fn list_session_worktrees_calls(&self) -> u64 {
        self.list_session_worktrees_calls.load(Ordering::SeqCst)
    }
    /// RPC-057: per-call counter — `inspect_session_changes`.
    pub fn inspect_session_changes_calls(&self) -> u64 {
        self.inspect_session_changes_calls.load(Ordering::SeqCst)
    }
    /// WT-009: per-call counter — `detach_session_worktree`.
    pub fn detach_session_worktree_calls(&self) -> u64 {
        self.detach_session_worktree_calls.load(Ordering::SeqCst)
    }
    /// RPC-057: seed the [`MergeOutcome`] returned by every subsequent
    /// `merge_session_worktree` call.
    pub fn seed_merge_outcome(&self, outcome: MergeOutcome) {
        {
            let mut g = stub_lock(&self.merge_outcome, "merge_outcome");
            *g = outcome;
        }
    }
    /// RPC-057: seed the pruned-session-id list returned by every
    /// subsequent `prune_orphaned_worktrees` call.
    pub fn seed_pruned_sessions(&self, ids: Vec<String>) {
        {
            let mut g = stub_lock(&self.pruned_sessions, "pruned_sessions");
            *g = ids;
        }
    }
    /// RPC-057: seed the session worktree rows returned by every
    /// subsequent `list_session_worktrees` call.
    pub fn seed_session_worktrees(&self, rows: Vec<SessionWorktreeInfo>) {
        {
            let mut g = stub_lock(&self.session_worktrees, "session_worktrees");
            *g = rows;
        }
    }
    /// RPC-057: seed the changes summary returned by every subsequent
    /// `inspect_session_changes` call.
    pub fn seed_session_changes_summary(&self, summary: SessionChangesSummary) {
        {
            let mut g = stub_lock(&self.session_changes_summary, "session_changes_summary");
            *g = summary;
        }
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule stub state + seeds.
    // ─────────────────────────────────────────────────────────────────
}
