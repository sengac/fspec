use codelet_rpc_types::{SessionId, SessionInfo, WorkUnitFocus};

use super::state::App;

impl App {
    pub(crate) fn handle_work_unit_session_created(&mut self, session: SessionId, card: String) {
        if !self.completed_story_creations.insert(session.clone()) {
            return;
        }
        self.initialize_session(session.clone());
        if let Some(index) = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|context| context.id == session)
        {
            self.switch_to_session_index(index);
        }
        if self.navigator.active_view == crate::views::ViewMode::Mux {
            self.navigator.mux.focus_agent_pane_for_session(&session);
        }
        self.attach_work_unit_to_session(session, card);
    }

    pub(crate) fn handle_legacy_session_attachment(&mut self, card: &str, session: &SessionId) {
        if self
            .agent_view_store
            .work_unit_focus_revision_for(session)
            .is_none()
        {
            self.board_store.attach_session(card, session.clone());
        }
    }

    pub(crate) fn handle_work_unit_focus_changed(
        &mut self,
        session: SessionId,
        focus: WorkUnitFocus,
    ) {
        if self.agent_view_store.apply_work_unit_focus(session, focus) {
            self.rebuild_focus_occupancy();
        }
    }

    pub(crate) fn handle_session_observed(&mut self, info: SessionInfo) {
        let session = SessionId::new(info.id);
        self.initialize_session(session.clone());
        self.handle_work_unit_focus_changed(session, info.work_unit_focus);
        self.rebuild_focus_occupancy();
    }

    pub(crate) fn rebuild_focus_occupancy(&mut self) {
        self.agent_view_store
            .sync_work_unit_contexts(self.board_store.work_units());
        let bindings = self
            .agent_view_store
            .open_sessions()
            .iter()
            .filter_map(|session| {
                self.agent_view_store
                    .work_unit_context_for(&session.id)
                    .map(|context| (context.id.clone(), session.id.clone()))
            })
            .collect::<Vec<_>>();
        self.board_store.replace_focus_occupancy(bindings);
    }

    pub(crate) fn spawn_hydrate_work_unit_focus(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Ok(focus) = backend.get_work_unit_focus(session.clone()).await {
                let _ = sender.send(crate::components::Action::WorkUnitFocusChanged(
                    session, focus,
                ));
            }
        }));
    }
}
