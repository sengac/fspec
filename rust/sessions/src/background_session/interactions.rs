use super::*;

impl BackgroundSession {
    pub fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.current()
    }

    pub fn set_pause_state(&self, state: Option<PauseState>) {
        let is_paused = state.is_some();
        if let Err(error) = self.pause_state.set(state) {
            tracing::error!(%error, "Cannot update pause state");
            return;
        }
        if is_paused {
            self.set_status(SessionStatus::Paused);
        }
    }

    pub fn clear_pause_state(&self) {
        if let Err(error) = self.pause_state.set(None) {
            tracing::error!(%error, "Cannot clear pause state");
            return;
        }
        self.set_status(SessionStatus::Running);
    }

    fn blocking_recv_compat<T>(recv: impl FnOnce() -> T) -> T {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(recv)
            }
            _ => recv(),
        }
    }

    pub fn wait_for_pause_response(&self) -> PauseResponse {
        Self::blocking_recv_compat(|| {
            let rx = self
                .pause_response_rx
                .lock()
                .expect("pause_response_rx lock poisoned");
            rx.recv().unwrap_or(PauseResponse::Interrupted)
        })
    }

    pub fn send_pause_response(&self, response: PauseResponse) {
        if let Err(error) = self.pause_state.respond(None, || {
            self.pause_response_tx
                .send(response)
                .map_err(|_| "Pause response channel closed".into())
        }) {
            tracing::error!(%error, "Cannot answer pause");
            return;
        }
        self.set_status(SessionStatus::Running);
    }

    pub fn send_pause_response_checked(
        &self,
        request_id: &str,
        response: PauseResponse,
    ) -> Result<(), String> {
        self.pause_state.respond(Some(request_id), || {
            self.pause_response_tx
                .send(response)
                .map_err(|_| "Pause response channel closed".into())
        })?;
        self.set_status(SessionStatus::Running);
        Ok(())
    }

    pub fn interaction_snapshot(&self) -> Result<String, String> {
        let (pause_id, pause) = self.pause_state.snapshot()?;
        let (hitl_id, hitl) = self.hitl_request.snapshot()?;
        let pause = pause.map(|p| serde_json::json!({"kind":format!("{:?}",p.kind),"tool_name":p.tool_name,"message":p.message,"details":p.details}));
        serde_json::to_string(&serde_json::json!({"pause": {"request_id":pause_id,"state":pause}, "hitl": {"request_id":hitl_id,"state":hitl}})).map_err(|e| e.to_string())
    }

    pub fn get_pause_response_tx(&self) -> std::sync::mpsc::Sender<PauseResponse> {
        self.pause_response_tx.clone()
    }

    pub fn wait_for_fspec_response(&self) -> FspecResult {
        Self::blocking_recv_compat(|| {
            let rx = self
                .fspec_response_rx
                .lock()
                .expect("fspec_response_rx lock poisoned");
            rx.recv().unwrap_or_else(|_| FspecResult {
                success: false,
                data: String::new(),
                error: Some("Fspec response channel closed unexpectedly".to_string()),
                system_reminder: None,
                tool_call_id: String::new(),
            })
        })
    }

    pub fn send_fspec_result(&self, result: FspecResult) {
        if let Err(e) = self.fspec_response_tx.send(result) {
            tracing::error!(error = %e, "[FSPEC_SESSION] Failed to send fspec result");
        }
    }

    pub fn wait_for_hitl_response(&self) -> codelet_tools::request_user_input::HitlResponse {
        Self::blocking_recv_compat(|| {
            let rx = self
                .hitl_response_rx
                .lock()
                .expect("hitl_response_rx lock poisoned");
            rx.recv()
                .unwrap_or(codelet_tools::request_user_input::HitlResponse::Cancelled {
                    cancelled: true,
                })
        })
    }

    pub fn send_hitl_response(&self, response: codelet_tools::request_user_input::HitlResponse) {
        if let Err(error) = self.hitl_request.respond(None, || {
            self.hitl_response_tx
                .send(response)
                .map_err(|_| "HITL response channel closed".into())
        }) {
            tracing::error!(%error, "Cannot answer HITL");
        }
    }

    pub fn send_hitl_response_checked(
        &self,
        request_id: &str,
        response: codelet_tools::request_user_input::HitlResponse,
    ) -> Result<(), String> {
        self.hitl_request.respond(Some(request_id), || {
            self.hitl_response_tx
                .send(response)
                .map_err(|_| "HITL response channel closed".into())
        })
    }

    pub fn set_hitl_request(
        &self,
        request: Option<codelet_tools::request_user_input::HitlRequest>,
    ) {
        if let Err(error) = self.hitl_request.set(request) {
            tracing::error!(%error, "Cannot update HITL request");
        }
    }

    pub fn get_hitl_request(&self) -> Option<codelet_tools::request_user_input::HitlRequest> {
        self.hitl_request.current()
    }
}
