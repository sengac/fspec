macro_rules! compose_embedded_focus_supervision {
    ($($collected:tt)*) => {
        compose_embedded_interaction! {
            $($collected)*
    async fn get_work_unit_focus(
        &self,
        session_id: SessionId,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, super::BackendError> {
        self.client
            .get_work_unit_focus(context::current(), session_id)
            .await
            .map_err(|error| super::BackendError::WorkUnitFocus(error.to_string()))?
            .map_err(super::BackendError::WorkUnitFocus)
    }

    async fn set_work_unit_focus(
        &self,
        session_id: SessionId,
        work_unit_id: Option<String>,
        source: String,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, super::BackendError> {
        self.client
            .set_work_unit_focus(context::current(), session_id, work_unit_id, source)
            .await
            .map_err(|error| super::BackendError::WorkUnitFocus(error.to_string()))?
            .map_err(super::BackendError::WorkUnitFocus)
    }

    fn work_unit_focus_rx(
        &self,
    ) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)> {
        self.transport.work_unit_focus_rx()
    }

    async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        Ok(self
            .client
            .get_work_unit_context(context::current(), session_id)
            .await?)
    }

    async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        self.client
            .set_work_unit_context(context::current(), session_id, context)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        Ok(self
            .client
            .get_pending_input(context::current(), session_id)
            .await?)
    }

    async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> {
        self.client
            .set_pending_input(context::current(), session_id, text)
            .await?;
        Ok(())
    }

    async fn set_active_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .set_active_session(context::current(), session_id)
            .await?;
        Ok(())
    }

    async fn clear_active_session(&self) -> Result<()> {
        self.client.clear_active_session(context::current()).await?;
        Ok(())
    }

    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        Ok(self.client.get_active_session(context::current()).await?)
    }

    async fn get_effective_cwd(&self, session_id: SessionId) -> Result<String> {
        Ok(self
            .client
            .get_effective_cwd(context::current(), session_id)
            .await?)
    }

    async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(self
            .client
            .get_supervisors(context::current(), session_id)
            .await?)
    }

    async fn add_supervisor(
        &self,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<()> {
        self.client
            .add_supervisor(context::current(), subordinate_id, supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn remove_supervisor(&self, supervisor_id: SessionId) -> Result<()> {
        self.client
            .remove_supervisor(context::current(), supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_subordinate(&self, supervisor_id: SessionId) -> Result<Option<SessionId>> {
        Ok(self
            .client
            .get_subordinate(context::current(), supervisor_id)
            .await?)
    }

    async fn get_subordinates(&self, supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(self
            .client
            .get_subordinates(context::current(), supervisor_id)
            .await?)
    }

    async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        self.client
            .receive_incoming_message(context::current(), subordinate_id, message)
            .await?
            .map_err(anyhow::Error::msg)
    }
        }
    };
}
