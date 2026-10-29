macro_rules! compose_websocket_session_context {
    ($($collected:tt)*) => {
        compose_websocket_debug_approval! {
            $($collected)*
    async fn get_work_unit_focus(
        &self,
        session_id: SessionId,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .get_work_unit_focus(context::current(), session_id)
            .await
            .map_err(|error| BackendError::WorkUnitFocus(error.to_string()))?
            .map_err(BackendError::WorkUnitFocus)
    }

    async fn set_work_unit_focus(
        &self,
        session_id: SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_work_unit_focus(context::current(), session_id, work_unit_id, source)
            .await
            .map_err(|error| BackendError::WorkUnitFocus(error.to_string()))?
            .map_err(BackendError::WorkUnitFocus)
    }

    fn work_unit_focus_rx(
        &self,
    ) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)> {
        let Ok(guard) = self.client.try_read() else {
            return empty_broadcast_rx();
        };
        let Some(client) = guard.as_ref() else {
            return empty_broadcast_rx();
        };
        super::focus_events::subscribe(
            client.session_events_rx(),
            client.session_snapshot_rx(),
            client.client().clone(),
        )
    }

    async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_work_unit_context(context::current(), session_id)
            .await?)
    }

    async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_work_unit_context(context::current(), session_id, context)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_pending_input(context::current(), session_id)
            .await?)
    }

    async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_pending_input(context::current(), session_id, text)
            .await?;
        Ok(())
    }

    async fn set_active_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_active_session(context::current(), session_id)
            .await?;
        Ok(())
    }

    async fn clear_active_session(&self) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .clear_active_session(context::current())
            .await?;
        Ok(())
    }

    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_active_session(context::current())
            .await?)
    }

    async fn get_effective_cwd(&self, session_id: SessionId) -> Result<String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_effective_cwd(context::current(), session_id)
            .await?)
    }

    async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_supervisors(context::current(), session_id)
            .await?)
    }

    async fn add_supervisor(
        &self,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .add_supervisor(context::current(), subordinate_id, supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn remove_supervisor(&self, supervisor_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .remove_supervisor(context::current(), supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_subordinate(&self, supervisor_id: SessionId) -> Result<Option<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_subordinate(context::current(), supervisor_id)
            .await?)
    }

    async fn get_subordinates(&self, supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_subordinates(context::current(), supervisor_id)
            .await?)
    }

    async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .receive_incoming_message(context::current(), subordinate_id, message)
            .await?
            .map_err(anyhow::Error::msg)
    }
        }
    };
}
