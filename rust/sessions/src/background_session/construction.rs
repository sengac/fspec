use super::*;

impl BackgroundSession {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        name: String,
        project: String,
        provider_id: Option<String>,
        model_id: Option<String>,
        mut inner: codelet_cli::session::Session,
        input_tx: mpsc::Sender<PromptInput>,
        worktree_path: Option<PathBuf>,
        base_commit: Option<String>,
        lifecycle_hooks: Option<Arc<CompiledLifecycleHooks>>,
        chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>,
        status_changes_tx: broadcast::Sender<(
            codelet_rpc_types::SessionId,
            codelet_rpc_types::SessionStatus,
        )>,
        session_cleared_tx: broadcast::Sender<codelet_rpc_types::SessionId>,
    ) -> Self {
        inner.provider_session.bind_live_session(id);
        let (incoming_message_tx, incoming_message_rx) = mpsc::channel::<IncomingMessage>(16);
        let (pause_response_tx, pause_response_rx) = std::sync::mpsc::channel::<PauseResponse>();
        let (fspec_response_tx, fspec_response_rx) = std::sync::mpsc::channel::<FspecResult>();
        let (hitl_response_tx, hitl_response_rx) =
            std::sync::mpsc::channel::<codelet_tools::request_user_input::HitlResponse>();
        Self {
            id,
            name: RwLock::new(name),
            provider_status: std::sync::Mutex::new(None),
            project,
            provider_id: RwLock::new(provider_id),
            model_id: RwLock::new(model_id),
            cached_input_tokens: AtomicU32::new(0),
            cached_output_tokens: AtomicU32::new(0),
            committed_context: std::sync::Mutex::new(Default::default()),
            cached_reasoning_tokens: AtomicU32::new(0),
            cached_context_window: AtomicU32::new(0),
            cached_max_output_tokens: AtomicU32::new(0),
            cached_compaction_threshold: AtomicU32::new(0),
            inner: Arc::new(Mutex::new(inner)),
            settlement_gate: Arc::new(Mutex::new(())),
            status: AtomicU8::new(SessionStatus::Idle as u8),
            input_tx,
            output_buffer: RwLock::new(crate::transcript_journal::TranscriptJournal::default()),
            transcript_seed: std::sync::Mutex::new(()),
            manifest_restore: AtomicBool::new(false),
            relay_output: crate::subordinate_output::RelayOutput::default(),
            is_interrupted: Arc::new(AtomicBool::new(false)),
            interrupt_epoch: AtomicU64::new(0),
            interruption_reason: std::sync::Mutex::new(None),
            interrupt_notify: Arc::new(Notify::new()),
            is_debug_enabled: AtomicBool::new(false),
            pending_input: RwLock::new(None),
            supervisor_broadcast: broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY).0,
            chunks_tx,
            status_changes_tx,
            session_cleared_tx,
            role: RwLock::new(None),
            incoming_message_tx,
            incoming_message_rx: Mutex::new(incoming_message_rx),
            incoming_message_pending: Arc::new(AtomicUsize::new(0)),
            correlation_counter: AtomicU64::new(0),
            pending_observed_correlation_ids: RwLock::new(Vec::new()),
            pause_state: crate::interaction_slot::InteractionSlot::new(),
            pause_response_tx,
            pause_response_rx: std::sync::Mutex::new(pause_response_rx),
            fspec_response_tx,
            fspec_response_rx: std::sync::Mutex::new(fspec_response_rx),
            hitl_response_tx,
            hitl_response_rx: std::sync::Mutex::new(hitl_response_rx),
            hitl_request: crate::interaction_slot::InteractionSlot::new(),
            exec_stdin_request: RwLock::new(None),
            base_thinking_level: AtomicU8::new(0),
            continue_enabled: AtomicBool::new(false),
            continue_budget: AtomicU32::new(10),
            continue_explicit: AtomicBool::new(false),
            goal_state: std::sync::Mutex::new(None),
            goal_generation: AtomicU64::new(0),
            goal_synced_generation: AtomicU64::new(0),
            compaction_progress: RwLock::new(None),
            work_unit_context: RwLock::new(None),
            work_unit_focus: Arc::new(std::sync::Mutex::new(Default::default())),
            focus_reminder_revision: Arc::new(AtomicU64::new(u64::MAX)),
            focus_admission: std::sync::Mutex::new(0),
            focus_runtime: tokio::runtime::Handle::current(),
            base_environment_content: RwLock::new(gather_environment_info().to_reminder_content()),
            worktree_path: RwLock::new(worktree_path),
            base_commit: RwLock::new(base_commit),
            compaction_in_progress: Arc::new(AtomicBool::new(false)),
            pending_dag_content: Arc::new(std::sync::Mutex::new(None)),
            pre_compaction_tokens: AtomicU32::new(0),
            schedule_triggered: AtomicBool::new(false),
            schedule_name: RwLock::new(None),
            lifecycle_hooks,
            owning_manager: RwLock::new(std::sync::Weak::new()),
            pending_loop_abort_note: std::sync::Mutex::new(None),
            debug_capture: {
                let mgr = DebugCaptureManager::new().unwrap_or_else(|e| {
                    tracing::warn!("Failed to create per-session DebugCaptureManager: {e}");
                    DebugCaptureManager::new()
                        .expect("DebugCaptureManager::new() failed twice - data dir not set")
                });
                Arc::new(PoisonRecoveryMutex::new(mgr))
            },
        }
    }
}
