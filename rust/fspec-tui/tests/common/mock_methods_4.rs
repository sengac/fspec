//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn write_exec_stdin(
        &self,
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String> {
        self.controls
            .write_exec_stdin_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .write_exec_stdin_calls_log
            .lock()
            .push((session_id, exec_session_id, text));
        if let Some(msg) = self.controls.write_exec_stdin_error.lock().clone() {
            return Err(msg);
        }
        Ok(())
    }
    pub async fn pause_human_action(
        &self,
        session_id: SessionId,
        request_id: String,
        reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<()> {
        self.record_reply("pause_human_action", &session_id, request_id);
        self.controls
            .pause_human_action_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .pause_human_action_calls_log
            .lock()
            .push((session_id, reply));
        if let Some(msg) = self.controls.pause_human_action_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn pause_confirm(
        &self,
        session_id: SessionId,
        request_id: String,
        accept: bool,
    ) -> Result<()> {
        self.record_reply("pause_confirm", &session_id, request_id);
        self.controls
            .pause_confirm_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .pause_confirm_calls_log
            .lock()
            .push((session_id, accept));
        if let Some(msg) = self.controls.pause_confirm_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn pause_triple(
        &self,
        session_id: SessionId,
        request_id: String,
        choice: ApprovalChoice,
    ) -> Result<()> {
        self.record_reply("pause_triple", &session_id, request_id);
        self.controls
            .pause_triple_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .pause_triple_calls_log
            .lock()
            .push((session_id, choice));
        if let Some(msg) = self.controls.pause_triple_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn send_hitl_response(
        &self,
        session_id: SessionId,
        request_id: String,
        response: HitlResponse,
    ) -> Result<()> {
        self.record_reply("send_hitl_response", &session_id, request_id);
        self.controls
            .send_hitl_response_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .send_hitl_response_calls_log
            .lock()
            .push((session_id, response));
        if let Some(msg) = self.controls.send_hitl_response_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    // ── RPC-054 provider-credentials surface ─────────────────────────────

    pub async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        // BUG-575: read now, answer when a gate (if any) releases the call.
        let listed = self.provider_credentials_now();
        self.gates.pass("list_provider_credentials").await;
        listed
    }

    fn provider_credentials_now(&self) -> Result<Vec<ProviderCredentialInfo>> {
        self.controls
            .list_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.list_provider_credentials_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // One-shot overrides take priority: after-save → after-delete →
        // after-refresh → next_list_override → seeded list.
        if let Some(list) = self.controls.provider_credentials_after_save.lock().take() {
            *self.controls.provider_credentials.lock() = list.clone();
            return Ok(list);
        }
        if let Some(list) = self
            .controls
            .provider_credentials_after_delete
            .lock()
            .take()
        {
            *self.controls.provider_credentials.lock() = list.clone();
            return Ok(list);
        }
        if let Some(list) = self
            .controls
            .provider_credentials_after_refresh
            .lock()
            .take()
        {
            *self.controls.provider_credentials.lock() = list.clone();
            return Ok(list);
        }
        if let Some(list) = self.controls.next_list_override.lock().take() {
            *self.controls.provider_credentials.lock() = list.clone();
            return Ok(list);
        }
        Ok(self.controls.provider_credentials.lock().clone())
    }
}
