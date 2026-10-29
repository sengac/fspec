macro_rules! compose_service_supervision_credentials {
    ($($collected:tt)*) => {
        compose_service_finish! {
            $($collected)*
    /// RPC-037: list supervisor session ids for a subordinate.
    async fn get_supervisors(session_id: SessionId) -> Vec<SessionId>;

    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. The production handle delegates into
    /// `ChainOfCommand::add_supervisor` and surfaces its
    /// "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" error strings verbatim.
    async fn add_supervisor(
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<(), String>;

    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor.
    async fn remove_supervisor(supervisor_id: SessionId) -> Result<(), String>;

    /// RPC-061: return the first subordinate registered to a
    /// supervisor. Backward-compatible accessor mirroring
    /// `ChainOfCommand::get_subordinate`.
    async fn get_subordinate(supervisor_id: SessionId) -> Option<SessionId>;

    /// RPC-061: list every subordinate of a supervisor. Mirrors
    /// `ChainOfCommand::get_subordinates`.
    async fn get_subordinates(supervisor_id: SessionId) -> Vec<SessionId>;

    /// RPC-061: queue an incoming supervisor message for a subordinate
    /// session. Production handle wraps this onto
    /// `BackgroundSession::receive_incoming_message`; bubbles the
    /// underlying `Err(String)` (e.g. "Failed to queue supervisor
    /// input: …") back to the caller.
    async fn receive_incoming_message(
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String>;

    /// RPC-037: debug-capture toggle reader.
    async fn get_debug_enabled(session_id: SessionId) -> bool;

    /// RPC-037: debug-capture toggle writer.
    async fn set_debug_enabled(session_id: SessionId, enabled: bool);

    /// RPC-037: toggle debug capture; returns the resolved path string.
    async fn toggle_debug(session_id: SessionId, debug_dir: String) -> Result<String, String>;

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Mirrors the NAPI
    /// `toggle_debug(Option<String>)` global helper.
    async fn set_debug_directory(path: String) -> Result<(), String>;

    /// RPC-037: resume a paused session.
    async fn pause_resume(session_id: SessionId) -> Result<(), String>;

    /// RPC-037: respond to a two-choice confirm pause.
    async fn pause_confirm(session_id: SessionId, accept: bool) -> Result<(), String>;

    /// RPC-037: respond to a three-choice approval pause.
    async fn pause_triple(session_id: SessionId, choice: ApprovalChoice) -> Result<(), String>;

    /// RPC-037: send a Human-In-The-Loop response.
    async fn send_hitl_response(
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<(), String>;

    /// RPC-037: snapshot of the pause dialog state.
    async fn get_pause_state(session_id: SessionId) -> Option<PauseState>;

    /// RPC-037: snapshot of the active HITL request, if any.
    async fn get_hitl_request(session_id: SessionId) -> Option<HitlRequest>;

    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure overlay — NO status flip, NO response channel.
    async fn get_exec_stdin_request(session_id: SessionId) -> Option<ExecStdinRequest>;

    /// TOOL-022 P2: write typed text to a live exec session's stdin
    /// (a trailing newline is appended when absent, matching the
    /// unified_exec `write` action semantics).
    async fn write_exec_stdin(
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String>;

    /// RPC-037: round-trip an FspecCommandRequest reply.
    async fn send_fspec_result(session_id: SessionId, result: FspecResult) -> Result<(), String>;

    /// RPC-037: create an isolated (worktree-backed) session.
    async fn create_isolated_session(role: Option<String>) -> Result<IsolatedSessionInfo, String>;

    /// RPC-037: per-user default thinking level (closes the pre-RPC-037
    /// gap on the tarpc surface).
    async fn set_thinking_level_default(
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String>;

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    async fn destroy_session(session_id: SessionId) -> Result<(), String>;

    // ========================================================================
    // RPC-054: Provider credentials surface — backs the new Rust ratatui
    // ProviderSettingsView (`/provider` slash command). Mirrors the
    // SessionManagerHandle trait additions of the same name.
    // ========================================================================

    /// RPC-054: list all known providers with configured / credential-type
    /// / model-count metadata.
    async fn list_provider_credentials() -> Vec<ProviderCredentialInfo>;

    /// RPC-054: return the credential summary for a single provider.
    async fn get_provider_credential(provider_id: String) -> Option<ProviderCredentialInfo>;

    /// RPC-054: persist credentials for a provider.
    async fn set_provider_credentials(
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<(), String>;

    /// RPC-054: clear credentials for a provider. Idempotent.
    async fn delete_provider_credentials(provider_id: String) -> Result<(), String>;

    /// RPC-054: perform a network round-trip to the provider's base
    /// URL and return latency + success metadata.
    async fn test_provider_connection(provider_id: String) -> Result<TestConnectionResult, String>;

    /// RPC-054: refresh the provider's cached model list and return
    /// the fresh `ModelEntry` list.
    async fn refresh_models_cache(provider_id: String) -> Result<Vec<ModelEntry>, String>;

    /// PROV-112: clear the OAuth tokens for `provider_id` (disconnect/logout).
    /// Dispatches by provider to the providers-direct clear primitives
    /// (anthropic→delete claude_auth.json, github-copilot→delete copilot
    /// credential, codex/fallback→strip the tokens field preserving
    /// OPENAI_API_KEY). Idempotent.
    async fn oauth_clear_tokens(provider_id: String) -> Result<(), String>;

    /// PROV-112: whether `provider_id` currently has OAuth tokens persisted.
    /// Drives the post-disconnect nav reload (the `oauth-status` row is shown
    /// only while this is `true`).
    async fn oauth_get_tokens(provider_id: String) -> Result<bool, String>;

    /// PROV-113: run the browser OAuth login for `provider_id` to completion
    /// (providers-layer local-server flow; tokens persisted on success).
    async fn oauth_browser_login(provider_id: String) -> Result<(), String>;

    /// PROV-113: phase 1 of the anthropic headless flow — generate PKCE +
    /// authorize URL (no network).
    async fn oauth_headless_start(provider_id: String) -> Result<OAuthHeadlessStart, String>;

    /// PROV-113: phase 2 of the anthropic headless flow — validate the pasted
    /// `code#state`, exchange for tokens, persist.
    async fn oauth_headless_complete(
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<(), String>;

    /// PROV-113: phase 1 of the codex device flow — request a device code.
    async fn oauth_device_start(provider_id: String) -> Result<OAuthDeviceStart, String>;

    /// PROV-113: phase 2 of the codex device flow — poll until authorized,
    /// then exchange + persist.
    async fn oauth_device_poll(
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<(), String>;

    /// PROV-114: phase 1 of the github-copilot device flow — request a device
    /// code against github.com (`None`) or a normalized enterprise host.
    async fn oauth_copilot_device_start(
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart, String>;

    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the left pane of
    /// `BlocklistView` in the Rust ratatui frontend.
    async fn blocklist_list() -> Vec<BlocklistRuleInfo>;

    /// RPC-057: merge a session's worktree back to base. Strategy is
    /// reserved for future evolution.
    async fn merge_session_worktree(
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String>;

    /// RPC-057: discard a session's worktree changes.
    async fn discard_session_worktree(session_id: SessionId) -> Result<(), String>;

    /// RPC-057: prune orphaned session worktrees; returns the pruned
    /// session ids.
    async fn prune_orphaned_worktrees() -> Result<Vec<String>, String>;

    /// RPC-057: list every known session worktree.
    async fn list_session_worktrees() -> Vec<SessionWorktreeInfo>;

    /// RPC-057: inspect a session's pending change summary.
    async fn inspect_session_changes(
        session_id: SessionId,
    ) -> Result<SessionChangesSummary, String>;

    /// WT-009: detach a session from its isolation worktree (clears the
    /// session's worktree path/base commit, deletes the git-session
    /// manifest, re-spawns the footer poller with the project cwd, emits
    /// IsolationStateChange(false, None)). The session keeps running.
    async fn detach_session_worktree(session_id: SessionId) -> Result<(), String>;

    /// RPC-058: persist a new scheduled job.
    async fn schedule_add(job: ScheduledJob) -> Result<ScheduledJob, String>;

    /// RPC-058: list every persisted scheduled job.
    async fn schedule_list() -> Vec<ScheduledJob>;

    /// RPC-058: flip a job's status to `paused`.
    async fn schedule_pause(name: String) -> Result<ScheduledJob, String>;

    /// RPC-058: flip a job's status to `active`.
    async fn schedule_resume(name: String) -> Result<ScheduledJob, String>;

    /// RPC-058: remove a job from `spec/schedules.json`.
    async fn schedule_remove(name: String) -> Result<(), String>;

    /// RPC-059: register a session-scoped recurring prompt.
    async fn loop_add(
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String>;

    /// RPC-059: cancel a registered loop. Returns true when a matching
    /// loop existed and was removed.
    async fn loop_cancel(id: String) -> Result<bool, String>;

    /// RPC-059: list every loop registered against a session.
    async fn loop_list(session_id: SessionId) -> Vec<RegisteredLoop>;

    async fn get_transcript_page(
        session_id: SessionId,
        cursor: Option<codelet_rpc_types::TranscriptCursor>,
        snapshot_end: Option<u64>,
        max_bytes: u32,
    ) -> Result<codelet_rpc_types::TranscriptPage, codelet_rpc_types::TranscriptError>;

    /// Appended wire variant; requires read-v2 handshake negotiation.
    async fn read_work_units_bounded(detail: bool, arguments: String) -> Result<String, String>;
    /// Snapshot request identities with their pending interaction payloads.
    async fn get_interaction_snapshot(session_id: SessionId) -> Result<String, String>;
    /// Atomic request-bound replacement for legacy pause/HITL responses.
    async fn reply_interaction_checked(
        session_id: SessionId,
        method: String,
        request_id: String,
        arguments: String,
    ) -> Result<(), String>;
    async fn create_session_checked(role: Option<String>) -> Result<SessionId, String>;
    async fn session_manifest_issues(
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>, String>;
        }
    };
}
