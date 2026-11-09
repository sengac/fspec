macro_rules! compose_rpc_session_restore_focus {
    ($($collected:tt)*) => {
        compose_rpc_supervision_interaction! {
            $($collected)*
    async fn clear_history(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.clear_history(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn compact_session(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<CompactionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.compact_session(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn restore_session_messages(
        self,
        _ctx: Context,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.restore_session_messages(&session_id, envelopes),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn restore_session_token_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.restore_session_token_state(&session_id, state),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn resume_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.resume_session(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_work_unit_focus(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<WorkUnitFocus, String> {
        self.inner
            .session_manager()
            .ok_or_else(|| "Session manager unavailable".to_string())?
            .get_work_unit_focus(&session_id)
    }

    async fn set_work_unit_focus(
        self,
        _ctx: Context,
        session_id: SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> Result<WorkUnitFocus, String> {
        self.inner
            .session_manager()
            .ok_or_else(|| "Session manager unavailable".to_string())?
            .set_work_unit_focus(&session_id, work_unit_id, source)
    }

    async fn get_work_unit_context(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_work_unit_context(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_work_unit_context(
        self,
        _ctx: Context,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_work_unit_context(&session_id, context),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_pending_input(self, _ctx: Context, session_id: SessionId) -> Result<Option<String>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pending_input(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_pending_input(self, _ctx: Context, session_id: SessionId, text: Option<String>) -> Result<(), String> {
        // BUG-464: no session manager and an unknown session are errors.
        match self.inner.session_manager() {
            Some(handle) => handle.set_pending_input(&session_id, text),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_active_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        // BUG-465: no session manager is refused, not a silent no-op.
        match self.inner.session_manager() {
            Some(handle) => handle.set_active_session(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn clear_active_session(self, _ctx: Context) -> Result<(), String> {
        // BUG-465: no session manager is refused, not a silent no-op.
        match self.inner.session_manager() {
            Some(handle) => handle.clear_active_session(),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_active_session(self, _ctx: Context) -> Result<Option<SessionId>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_active_session(),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_effective_cwd(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<String, String> {
        match self.inner.session_manager() {
            Some(handle) => handle
                .get_effective_cwd(&session_id)
                .map(|cwd| cwd.to_string_lossy().into_owned()),
            None => Err("no session manager is attached to resolve the effective cwd".to_string()),
        }
    }
        }
    };
}
