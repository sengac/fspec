macro_rules! compose_rpc_schedule_loops {
    ($($collected:tt)*) => {
        compose_rpc_finish! {
            $($collected)*
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(self, _ctx: Context, job: ScheduledJob) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_add(job),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_list(self, _ctx: Context) -> Vec<ScheduledJob> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_list(),
            None => Vec::new(),
        }
    }

    async fn schedule_pause(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_pause(&name),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_resume(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_resume(&name),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_remove(self, _ctx: Context, name: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_remove(&name),
            None => Ok(()),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        self,
        _ctx: Context,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_add(&session_id, interval_seconds, prompt),
            None => Ok(RegisteredLoop::default()),
        }
    }

    async fn loop_cancel(self, _ctx: Context, id: String) -> Result<bool, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_cancel(&id),
            None => Ok(false),
        }
    }

    async fn loop_list(self, _ctx: Context, session_id: SessionId) -> Vec<RegisteredLoop> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_list(&session_id),
            None => Vec::new(),
        }
    }
        }
    };
}
