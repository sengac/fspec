macro_rules! compose_rpc_session_restore_focus {
    ($($collected:tt)*) => {
        compose_rpc_supervision_interaction! {
            $($collected)*
    async fn clear_history(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.clear_history(&session_id),
            None => Ok(()),
        }
    }

    async fn compact_session(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<CompactionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.compact_session(&session_id),
            None => Ok(CompactionResult {
                original_tokens: 0,
                compacted_tokens: 0,
                compression_ratio: 0.0,
                turns_summarized: 0,
                turns_kept: 0,
            }),
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
            None => Ok(()),
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
            None => Ok(()),
        }
    }

    async fn resume_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.resume_session(&session_id),
            None => Ok(()),
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
    ) -> Option<WorkUnitContext> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_work_unit_context(&session_id),
            None => None,
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
            None => Ok(()),
        }
    }

    async fn get_pending_input(self, _ctx: Context, session_id: SessionId) -> Option<String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pending_input(&session_id),
            None => None,
        }
    }

    async fn set_pending_input(self, _ctx: Context, session_id: SessionId, text: Option<String>) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_pending_input(&session_id, text);
        }
    }

    async fn set_active_session(self, _ctx: Context, session_id: SessionId) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_active_session(&session_id);
        }
    }

    async fn clear_active_session(self, _ctx: Context) {
        if let Some(handle) = self.inner.session_manager() {
            handle.clear_active_session();
        }
    }

    async fn get_active_session(self, _ctx: Context) -> Option<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_active_session(),
            None => None,
        }
    }

    async fn get_effective_cwd(self, _ctx: Context, session_id: SessionId) -> String {
        match self.inner.session_manager() {
            Some(handle) => handle
                .get_effective_cwd(&session_id)
                .to_string_lossy()
                .into_owned(),
            None => String::new(),
        }
    }
        }
    };
}
