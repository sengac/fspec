use super::*;

impl BackgroundSession {
    pub(crate) fn base_environment_for_focus(&self) -> String {
        self.base_environment_content
            .read()
            .expect("base environment lock poisoned")
            .clone()
    }

    pub fn get_work_unit_context(&self) -> Option<WorkUnitContext> {
        self.work_unit_context
            .read()
            .expect("work_unit_context lock poisoned")
            .clone()
    }

    pub fn set_work_unit_context(
        &self,
        id: Option<String>,
        title: Option<String>,
        status: Option<String>,
    ) {
        let owning_manager = self.owning_manager();
        let manager = owning_manager
            .as_deref()
            .unwrap_or_else(|| crate::SessionManager::instance());
        if manager.get_session(&self.id.to_string()).is_ok() {
            if let Err(error) = manager.set_session_focus(
                &codelet_rpc_types::SessionId::from(self.id.to_string()),
                id.as_deref(),
                "board",
            ) {
                tracing::warn!(%error, "could not update session focus");
                return;
            }
        }
        if let (Some(id), Some(title), Some(status)) = (id, title, status) {
            let authorization = codelet_rpc_types::WorkUnitContext {
                id: id.clone(),
                title: title.clone(),
                status: status.clone(),
            };
            if let Err(error) =
                codelet_core::persistence::update_session_manifest(self.id, |manifest| {
                    manifest.work_unit_authorization = Some(authorization);
                    Ok(())
                })
            {
                tracing::warn!(%error, "could not persist authorization binding");
                return;
            }
            *self
                .work_unit_context
                .write()
                .expect("work_unit_context lock poisoned") =
                Some(WorkUnitContext::new(id, title, status));
        }
        codelet_tools::broadcast_metadata_update();
    }

    pub(crate) fn restore_work_unit_authorization(
        &self,
        context: Option<codelet_rpc_types::WorkUnitContext>,
    ) {
        *self
            .work_unit_context
            .write()
            .expect("authorization lock poisoned") =
            context.map(|context| WorkUnitContext::new(context.id, context.title, context.status));
    }
}
