//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn update_custom_model(
        &self,
        _provider_id: String,
        _profile_name: String,
        _original_model_id: String,
        _definition: codelet_rpc_types::CustomModelDefinition,
    ) -> Result<()> {
        self.custom_model_write_result()
    }
    pub async fn delete_custom_model(
        &self,
        _provider_id: String,
        _profile_name: String,
        _model_id: String,
    ) -> Result<()> {
        self.custom_model_write_result()
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn file_diff(&self, _path: String) -> Result<Option<String>> {
        Ok(None)
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn session_manifest_issues(
        &self,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>> {
        Ok(Vec::new())
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn clear_active_session(&self) -> Result<()> {
        Ok(())
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn get_effective_cwd(&self, _session_id: SessionId) -> Result<String> {
        Ok(String::new())
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn get_compaction_progress(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<codelet_rpc_types::CompactionProgress>> {
        Ok(None)
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn get_session_model(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::SessionModel> {
        Ok(codelet_rpc_types::SessionModel {
            provider_id: String::new(),
            model_id: String::new(),
            context_window: None,
            max_output_tokens: None,
            compaction_threshold: None,
        })
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn get_session_tokens(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::SessionTokens> {
        Ok(codelet_rpc_types::SessionTokens {
            input_tokens: 0,
            output_tokens: 0,
        })
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn restore_session_messages(
        &self,
        _session_id: SessionId,
        _envelopes: Vec<String>,
    ) -> Result<()> {
        Ok(())
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn restore_session_token_state(
        &self,
        _session_id: SessionId,
        _state: codelet_rpc_types::TokenRestoreState,
    ) -> Result<()> {
        Ok(())
    }
    // BUG-453: the mock states its answer explicitly (the trait default refuses).
    pub async fn get_default_model(&self) -> Result<Option<String>> {
        Ok(None)
    }
    pub async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>> {
        self.core.list_sessions_calls.fetch_add(1, Ordering::SeqCst);
        *self.core.last_list_sessions_project.lock() = Some(project_path);
        let listed = match self.backend_state.list_sessions_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(self.core.sessions.lock().clone()),
        };
        self.gates.pass("list_sessions").await;
        listed
    }
    // BUG-466: the trait defaults refuse; this fake backend stores the state.
    pub async fn get_continue_state(&self, _session_id: SessionId) -> Result<(bool, u32)> {
        Ok(*self.backend_state.continue_state.lock())
    }
    pub async fn set_continue_state(
        &self,
        _session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<()> {
        *self.backend_state.continue_state.lock() = (enabled, budget);
        Ok(())
    }
    pub async fn get_goal_state(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<(String, Option<String>)>> {
        Ok(self.backend_state.goal_state.lock().clone())
    }
    pub async fn set_goal_state(
        &self,
        _session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<()> {
        *self.backend_state.goal_state.lock() = goal;
        Ok(())
    }
    pub async fn set_thinking_level_default(
        &self,
        _session_id: SessionId,
        level: codelet_rpc_types::ThinkingLevel,
    ) -> Result<()> {
        self.core
            .set_thinking_level_default_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_thinking_level_default.lock() = Some(level);
        Ok(())
    }
    pub async fn rename_profile(
        &self,
        _provider_id: String,
        _old_name: String,
        _new_name: String,
        _definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        Ok(())
    }
    pub async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        _thinking: Option<codelet_rpc_types::ThinkingConfig>,
    ) -> Result<()> {
        self.send_input(session_id, text).await
    }
}
