//! The one `impl FspecBackend for MockBackend`: every method delegates to
//! the inherent method of the same name in the `mock_methods_*` modules.

use super::*;

macro_rules! delegate_backend {
    (@call $name:ident) => { MockBackend::$name };
    (@call $name:ident $target:ident) => { MockBackend::$target };
    (
        async { $(fn $name:ident(&self $(, $arg:ident: $ty:ty)*) -> $ret:ty $(=> $target:ident)?;)* }
        sync { $(fn $sname:ident(&self $(, $sarg:ident: $sty:ty)*) -> $sret:ty;)* }
    ) => {
        #[async_trait]
        impl FspecBackend for MockBackend {
            $(async fn $name(&self $(, $arg: $ty)*) -> $ret {
                delegate_backend!(@call $name $($target)?)(self $(, $arg)*).await
            })*
            $(fn $sname(&self $(, $sarg: $sty)*) -> $sret {
                MockBackend::$sname(self $(, $sarg)*)
            })*
        }
    };
}

#[rustfmt::skip]
delegate_backend! {
async {
    fn list_provider_settings(&self) -> std::result::Result< Vec<codelet_rpc_types::ProviderSettingsConfig>, codelet_fspec_tui::transport::BackendError, >;
    fn save_provider_settings(&self, provider_id: String, definition: codelet_rpc_types::ProfileDefinition) -> std::result::Result<(), codelet_fspec_tui::transport::BackendError>;
    fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>;
    fn list_workspaces(&self) -> Result<Vec<codelet_rpc_types::WorkspaceEntry>>;
    fn list_workspace_work_units(&self, workspace: String) -> Result<Vec<WorkUnitInfo>>;
    fn create_session_in_workspace(&self, workspace: String, role: Option<String>) -> Result<SessionId>;
    fn list_live_sessions(&self) -> Result<Vec<SessionId>>;
    fn create_session(&self, role: Option<String>) -> Result<SessionId>;
    fn send_input(&self, id: SessionId, text: String) -> Result<()>;
    fn submit_input(&self, id: SessionId, text: String, mode: codelet_rpc_types::InputMode) -> Result<()>;
    fn configure_session_mcp_servers(&self, id: SessionId, servers: Vec<codelet_rpc_types::SessionMcpServer>) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>>;
    fn control_exec_stdin(&self, id: SessionId, exec_session_id: String, control: codelet_rpc_types::ExecStdinControl) -> Result<()>;
    fn interrupt(&self, id: SessionId) -> Result<()>;
    fn destroy_session(&self, session_id: SessionId) -> Result<()>;
    fn session_snapshot(&self) -> Vec<SessionInfo>;
    fn health(&self) -> Result<codelet_rpc_types::HealthInfo>;
    fn checkpoint_counts(&self) -> Result<CheckpointCounts>;
    fn restore_checkpoint_file(&self, work_unit_id: String, name: String, path: String) -> Result<()>;
    fn restore_checkpoint_all(&self, _work_unit_id: String, _name: String) -> Result<()>;
    fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()>;
    fn delete_all_checkpoints(&self) -> Result<()>;
    fn move_work_unit_up(&self, workspace: String, id: String) -> Result<()>;
    fn move_work_unit_down(&self, workspace: String, id: String) -> Result<()>;
    fn get_model_info(&self, _session_id: SessionId) -> Result<ModelInfo>;
    fn get_thinking_level(&self, _session_id: SessionId) -> Result<ThinkingLevel>;
    fn get_thinking_options(&self, _session_id: SessionId) -> Result<codelet_rpc_types::ThinkingOptions>;
    fn get_workspace_info(&self) -> Result<WorkspaceInfo>;
    fn changed_files(&self) -> Result<Vec<codelet_rpc_types::ChangedFile>>;
    fn list_checkpoints(&self) -> Result<Vec<codelet_rpc_types::CheckpointInfo>>;
    fn checkpoint_diff_files(&self, work_unit_id: String, name: String) -> Result<Vec<codelet_rpc_types::ChangedFile>>;
    fn checkpoint_file_diff(&self, work_unit_id: String, name: String, path: String) -> Result<Option<String>>;
    fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>>;
    fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>;
    fn persistence_get_history(&self, session: SessionId, _limit: u32) -> Result<Vec<String>>;
    fn persistence_search_history(&self, query: String) -> Result<Vec<codelet_rpc_types::HistoryMatch>>;
    fn persistence_delete_session(&self, id: SessionId) -> Result<()>;
    fn list_providers(&self) -> Result<Vec<ProviderInfo>>;
    fn set_session_model(&self, session_id: SessionId, provider_id: String, model_id: String) -> Result<()>;
    fn set_default_model(&self, model: String) -> Result<()>;
    fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> => set_session_thinking_level;
    fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>>;
    fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()>;
    fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()>;
    fn clear_history(&self, session_id: SessionId) -> Result<()>;
    fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult>;
    fn resume_session(&self, session_id: SessionId) -> Result<()>;
    fn get_transcript_page(&self, session_id: SessionId, cursor: Option<codelet_rpc_types::TranscriptCursor>, snapshot_end: Option<u64>, max_bytes: u32) -> std::result::Result< codelet_rpc_types::TranscriptPage, codelet_fspec_tui::transport::BackendError, >;
    fn get_buffered_output(&self, session_id: SessionId, limit: u32) -> Result<Vec<StreamChunk>>;
    fn get_active_session(&self) -> Result<Option<SessionId>>;
    fn set_active_session(&self, session_id: SessionId) -> Result<()>;
    fn get_work_unit_focus(&self, session: SessionId) -> std::result::Result< codelet_rpc_types::WorkUnitFocus, codelet_fspec_tui::transport::BackendError, >;
    fn set_work_unit_focus(&self, session: SessionId, work_unit_id: Option<String>, source: String) -> std::result::Result< codelet_rpc_types::WorkUnitFocus, codelet_fspec_tui::transport::BackendError, >;
    fn get_work_unit_context(&self, session_id: SessionId) -> Result<Option<WorkUnitContext>>;
    fn set_work_unit_context(&self, session_id: SessionId, context: Option<WorkUnitContext>) -> Result<()>;
    fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>>;
    fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()>;
    fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PendingRequest<PauseState>>>;
    fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<PendingRequest<HitlRequest>>>;
    fn get_exec_stdin_request(&self, session_id: SessionId) -> Result<Option<ExecStdinRequest>>;
    fn checkpoint_counts_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::CheckpointCounts>;
    fn list_checkpoints_in_workspace(&self, workspace: String) -> Result<Vec<codelet_rpc_types::CheckpointInfo>>;
    fn checkpoint_diff_files_in_workspace(&self, workspace: String, _work_unit_id: String, _name: String) -> Result<Vec<codelet_rpc_types::ChangedFile>>;
    fn checkpoint_file_diff_in_workspace(&self, workspace: String, _work_unit_id: String, _name: String, _path: String) -> Result<Option<String>>;
    fn restore_checkpoint_file_in_workspace(&self, workspace: String, _work_unit_id: String, _name: String, _path: String) -> Result<()>;
    fn restore_checkpoint_all_in_workspace(&self, workspace: String, _work_unit_id: String, _name: String) -> Result<()>;
    fn delete_checkpoint_in_workspace(&self, workspace: String, _work_unit_id: String, _name: String) -> Result<()>;
    fn delete_all_checkpoints_in_workspace(&self, workspace: String) -> Result<()>;
    fn write_exec_stdin(&self, session_id: SessionId, exec_session_id: String, text: String) -> Result<(), String>;
    fn pause_human_action(&self, session_id: SessionId, request_id: String, reply: codelet_rpc_types::HumanActionReply) -> Result<()>;
    fn pause_confirm(&self, session_id: SessionId, request_id: String, accept: bool) -> Result<()>;
    fn pause_triple(&self, session_id: SessionId, request_id: String, choice: ApprovalChoice) -> Result<()>;
    fn send_hitl_response(&self, session_id: SessionId, request_id: String, response: HitlResponse) -> Result<()>;
    fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>>;
    fn get_provider_credential(&self, provider_id: String) -> Result<Option<ProviderCredentialInfo>>;
    fn set_provider_credentials(&self, provider_id: String, creds: ProviderCredentialInput) -> Result<()>;
    fn delete_provider_credentials(&self, provider_id: String) -> Result<()>;
    fn save_profile(&self, provider_id: String, profile_name: String, definition: codelet_rpc_types::ProfileDefinition) -> Result<()>;
    fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()>;
    fn get_workspace_info_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::WorkspaceInfo>;
    fn git_state_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::GitState>;
    fn search_files_in_workspace(&self, workspace: String, prefix: String, limit: u32) -> Result<Vec<String>>;
    fn search_files_in_session(&self, session_id: SessionId, prefix: String, limit: u32) -> Result<Vec<String>>;
    fn blocklist_disabled_rules(&self, session_id: SessionId) -> Result<Vec<String>>;
    fn blocklist_set_rule_enabled(&self, session_id: SessionId, rule_id: String, enabled: bool) -> Result<Vec<String>>;
    fn persistence_add_history_in_workspace(&self, workspace: String, session: SessionId, text: String) -> Result<()>;
    fn persistence_get_history_in_workspace(&self, workspace: String, session: SessionId, limit: u32) -> Result<Vec<String>>;
    fn persistence_search_history_in_workspace(&self, workspace: String, query: String) -> Result<Vec<codelet_rpc_types::HistoryMatch>>;
    fn oauth_clear_tokens(&self, provider_id: String) -> Result<()>;
    fn oauth_get_tokens(&self, provider_id: String) -> Result<bool>;
    fn oauth_headless_start(&self, _provider_id: String) -> Result<codelet_rpc_types::OAuthHeadlessStart>;
    fn oauth_headless_complete(&self, provider_id: String, code_with_state: String, pkce_verifier: String) -> Result<()>;
    fn oauth_device_start(&self, provider_id: String) -> Result<codelet_rpc_types::OAuthDeviceStart>;
    fn oauth_device_poll(&self, _provider_id: String, _device_auth_id: String, _interval: u64) -> Result<()>;
    fn oauth_copilot_device_start(&self, enterprise_host: Option<String>) -> Result<codelet_rpc_types::OAuthDeviceStart>;
    fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult>;
    fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>>;
    fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String>;
    fn add_workspace(&self, path: String) -> Result<codelet_rpc_types::WorkspaceEntry>;
    fn remove_workspace(&self, path: String) -> Result<()>;
    fn set_debug_directory(&self, path: String) -> Result<()>;
    fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool>;
    fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()>;
    fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>>;
    fn merge_session_worktree(&self, _session_id: SessionId, _strategy: codelet_rpc_types::MergeStrategy) -> Result<codelet_rpc_types::MergeOutcome>;
    fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()>;
    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>>;
    fn list_session_worktrees(&self) -> Result<Vec<codelet_rpc_types::SessionWorktreeInfo>>;
    fn inspect_session_changes(&self, _session_id: SessionId) -> Result<codelet_rpc_types::SessionChangesSummary>;
    fn session_card_changes(&self, session_id: SessionId) -> Result<Vec<codelet_rpc_types::SessionCardChange>>;
    fn move_session_cards_back(&self, session_id: SessionId) -> Result<Vec<codelet_rpc_types::CardMoveBack>>;
    fn detach_session_worktree(&self, _session_id: SessionId) -> Result<()>;
    fn schedule_add(&self, _job: codelet_rpc_types::ScheduledJob) -> Result<codelet_rpc_types::ScheduledJob>;
    fn schedule_list(&self) -> Result<Vec<codelet_rpc_types::ScheduledJob>>;
    fn schedule_pause(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob>;
    fn schedule_resume(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob>;
    fn schedule_remove(&self, _name: String) -> Result<()>;
    fn loop_add(&self, _session_id: SessionId, _interval_seconds: u32, _prompt: String) -> Result<codelet_rpc_types::RegisteredLoop>;
    fn loop_cancel(&self, _id: String) -> Result<bool>;
    fn changed_files_in_workspace(&self, workspace: String) -> Result<Vec<codelet_rpc_types::ChangedFile>>;
    fn file_diff_in_workspace(&self, _workspace: String, _path: String) -> Result<Option<String>>;
    fn loop_list(&self, _session_id: SessionId) -> Result<Vec<codelet_rpc_types::RegisteredLoop>>;
    fn create_isolated_session(&self, _role: Option<String>) -> Result<IsolatedSessionInfo>;
    fn read_card_attachment(&self, workspace: String, path: String) -> Result<Vec<u8>>;
    fn create_isolated_session_in_workspace(&self, workspace: String, role: Option<String>) -> Result<IsolatedSessionInfo>;
    fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>>;
    fn add_supervisor(&self, _subordinate_id: SessionId, _supervisor_id: SessionId) -> Result<()>;
    fn remove_supervisor(&self, _supervisor_id: SessionId) -> Result<()>;
    fn get_subordinate(&self, _supervisor_id: SessionId) -> Result<Option<SessionId>>;
    fn get_subordinates(&self, _supervisor_id: SessionId) -> Result<Vec<SessionId>>;
    fn receive_incoming_message(&self, subordinate_id: SessionId, message: IncomingMessageInput) -> Result<()>;
    fn get_work_unit_details(&self, workspace: String, id: String) -> std::result::Result<String, codelet_fspec_tui::transport::BackendError>;
    fn add_custom_model(&self, _provider_id: String, _profile_name: String, _definition: codelet_rpc_types::CustomModelDefinition) -> Result<()>;
    fn update_custom_model(&self, _provider_id: String, _profile_name: String, _original_model_id: String, _definition: codelet_rpc_types::CustomModelDefinition) -> Result<()>;
    fn delete_custom_model(&self, _provider_id: String, _profile_name: String, _model_id: String) -> Result<()>;
    fn file_diff(&self, _path: String) -> Result<Option<String>>;
    fn session_manifest_issues(&self) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>>;
    fn clear_active_session(&self) -> Result<()>;
    fn get_effective_cwd(&self, _session_id: SessionId) -> Result<String>;
    fn get_compaction_progress(&self, _session_id: SessionId) -> Result<Option<codelet_rpc_types::CompactionProgress>>;
    fn get_session_model(&self, _session_id: SessionId) -> Result<codelet_rpc_types::SessionModel>;
    fn get_session_tokens(&self, _session_id: SessionId) -> Result<codelet_rpc_types::SessionTokens>;
    fn restore_session_messages(&self, _session_id: SessionId, _envelopes: Vec<String>) -> Result<()>;
    fn restore_session_token_state(&self, _session_id: SessionId, _state: codelet_rpc_types::TokenRestoreState) -> Result<()>;
    fn get_default_model(&self) -> Result<Option<String>>;
    fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>>;
    fn get_continue_state(&self, _session_id: SessionId) -> Result<(bool, u32)>;
    fn set_continue_state(&self, _session_id: SessionId, enabled: bool, budget: u32) -> Result<()>;
    fn get_goal_state(&self, _session_id: SessionId) -> Result<Option<(String, Option<String>)>>;
    fn set_goal_state(&self, _session_id: SessionId, goal: Option<(String, Option<String>)>) -> Result<()>;
    fn set_thinking_level_default(&self, _session_id: SessionId, level: codelet_rpc_types::ThinkingLevel) -> Result<()>;
    fn rename_profile(&self, _provider_id: String, _old_name: String, _new_name: String, _definition: codelet_rpc_types::ProfileDefinition) -> Result<()>;
    fn send_input_with_thinking(&self, session_id: SessionId, text: String, _thinking: Option<codelet_rpc_types::ThinkingConfig>) -> Result<()>;
}
sync {
    fn workspace_work_units_rx(&self) -> broadcast::Receiver<codelet_rpc_types::WorkspaceWorkUnits>;
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)>;
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo>;
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId>;
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId>;
    fn checkpoints_progress_rx(&self) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress>;
    fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState>;
    fn work_unit_focus_rx(&self) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)>;
}
}
