use codelet_rpc_types::{SessionId, WorkUnitFocus};

use super::AgentViewStore;

impl AgentViewStore {
    pub(crate) fn restore_work_unit_focus_projection(&mut self, session: &SessionId) {
        if let Some(focus) = self.work_unit_focus_by_session.get(session) {
            match focus.context.clone() {
                Some(context) => self.set_work_unit_context(session.clone(), context),
                None => self.clear_work_unit_context(session),
            }
        }
    }

    pub fn work_unit_focus_revision_for(&self, session: &SessionId) -> Option<u64> {
        self.work_unit_focus_by_session
            .get(session)
            .map(|focus| focus.revision)
    }

    pub fn apply_work_unit_focus(&mut self, session: SessionId, focus: WorkUnitFocus) -> bool {
        if self
            .work_unit_focus_revision_for(&session)
            .is_some_and(|revision| revision >= focus.revision)
        {
            return false;
        }
        self.work_unit_focus_by_session
            .insert(session.clone(), focus.clone());
        match focus.context {
            Some(context) => self.set_work_unit_context(session, context),
            None => self.clear_work_unit_context(&session),
        }
        true
    }
}
