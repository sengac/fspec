use super::*;

impl BackgroundSession {
    /// BUG-581: the environment the focus reminder rewrites, gathered now
    /// from the session's working directory and isolation context (a value
    /// cached at construction named the daemon's cwd and dropped an
    /// isolated session's isolation lines).
    pub(crate) fn base_environment_for_focus(&self) -> String {
        let isolation = self.build_isolation_context();
        gather_environment_info_with_isolation(&self.effective_cwd(), isolation.as_ref())
            .to_reminder_content()
    }

    pub fn get_work_unit_context(&self) -> Option<WorkUnitContext> {
        self.work_unit_context
            .read()
            .expect("work_unit_context lock poisoned")
            .clone()
    }

    /// Bind (or clear) the session's work unit: the owning manager's focus
    /// is updated, the authorization persisted to the session manifest and
    /// the in-memory context replaced. BUG-405: a missing owning manager, a
    /// failed focus update or a failed manifest write is returned, never
    /// logged and skipped.
    pub fn set_work_unit_context(
        &self,
        id: Option<String>,
        title: Option<String>,
        status: Option<String>,
    ) -> Result<(), String> {
        let manager = self
            .owning_manager()
            .ok_or_else(|| format!("session {} has no owning session manager", self.id))?;
        manager.get_session(&self.id.to_string())?;
        manager
            .set_session_focus(
                &codelet_rpc_types::SessionId::from(self.id.to_string()),
                id.as_deref(),
                "board",
            )
            .map_err(|error| format!("could not update session focus: {error}"))?;
        if let (Some(id), Some(title), Some(status)) = (id, title, status) {
            let authorization = codelet_rpc_types::WorkUnitContext {
                id: id.clone(),
                title: title.clone(),
                status: status.clone(),
            };
            codelet_core::persistence::update_session_manifest(self.id, |manifest| {
                manifest.work_unit_authorization = Some(authorization);
                Ok(())
            })
            .map_err(|error| format!("could not persist authorization binding: {error}"))?;
            *self
                .work_unit_context
                .write()
                .map_err(|_| "the work_unit_context lock is poisoned".to_string())? =
                Some(WorkUnitContext::new(id, title, status));
        }
        codelet_tools::broadcast_metadata_update();
        Ok(())
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
