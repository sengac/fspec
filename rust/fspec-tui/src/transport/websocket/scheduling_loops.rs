macro_rules! compose_websocket_scheduling_loops {
    ($($collected:tt)*) => {
        compose_websocket_finish! {
            $($collected)*
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_add(context::current(), job)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().schedule_list(context::current()).await?)
    }

    async fn schedule_pause(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_pause(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_resume(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_resume(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_remove(&self, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_remove(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_add(context::current(), session_id, interval_seconds, prompt)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_cancel(&self, id: String) -> Result<bool> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_cancel(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_list(&self, session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .loop_list(context::current(), session_id)
            .await?)
    }
        }
    };
}
