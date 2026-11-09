//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn persistence_search_history(
        &self,
        query: String,
    ) -> Result<Vec<codelet_rpc_types::HistoryMatch>> {
        self.core
            .search_history_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_history_query.lock() = Some(query);
        Ok(self.core.history_search_results.lock().clone())
    }
    pub async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        self.core
            .delete_session_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_deleted_session.lock() = Some(id.clone());
        let mut sessions = self.core.sessions.lock();
        sessions.retain(|s| s.id != id.value);
        Ok(())
    }
    pub async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        self.core
            .list_providers_calls
            .fetch_add(1, Ordering::SeqCst);
        self.core.call_log.lock().push("list_providers".to_string());
        let listed = match self.core.list_providers_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(self.core.providers.lock().clone()),
        };
        self.gates.pass("list_providers").await;
        listed
    }
    pub async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        self.core
            .set_session_model_calls
            .fetch_add(1, Ordering::SeqCst);
        let failure = self
            .model_selections
            .record(&session_id, &provider_id, &model_id);
        *self.core.last_set_session_model.lock() = Some((session_id, provider_id, model_id));
        match failure {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }
    pub async fn set_default_model(&self, model: String) -> Result<()> {
        self.core
            .set_default_model_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_default_model.lock() = Some(model);
        Ok(())
    }
    pub async fn set_session_thinking_level(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        self.core
            .set_thinking_level_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_thinking_level.lock() = Some((session_id, level));
        Ok(())
    }
    pub async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        self.core
            .get_session_role_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_get_session_role.lock() = Some(session_id.clone());
        let roles = self.core.session_roles.lock();
        let role = roles
            .iter()
            .find(|(s, _)| s == &session_id)
            .and_then(|(_, r)| r.clone());
        Ok(role)
    }
    pub async fn set_session_role(
        &self,
        session_id: SessionId,
        role: Option<String>,
    ) -> Result<()> {
        self.core
            .set_session_role_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_set_session_role.lock() = Some((session_id.clone(), role.clone()));
        // Mirror the SessionManager production behaviour: overwrite the
        // overlay when set, drop it when cleared.
        let mut roles = self.core.session_roles.lock();
        roles.retain(|(s, _)| s != &session_id);
        roles.push((session_id, role));
        Ok(())
    }
    pub async fn send_fspec_result(
        &self,
        session_id: SessionId,
        result: FspecResult,
    ) -> Result<()> {
        self.core
            .send_fspec_result_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_fspec_result.lock() = Some((session_id, result));
        Ok(())
    }
    pub async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        self.core.clear_history_calls.fetch_add(1, Ordering::SeqCst);
        *self.core.last_clear_history_session.lock() = Some(session_id);
        if let Some(msg) = self.core.clear_history_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
}
