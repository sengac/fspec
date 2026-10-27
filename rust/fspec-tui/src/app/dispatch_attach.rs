use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::store::SessionContext;
use crate::views::ViewMode;

use super::state::App;

impl App {
    fn focus_transcript_session(&mut self, session: &SessionId) {
        self.navigator.agent.resume_view = None;
        let existing = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|context| &context.id == session);
        match existing {
            Some(index) => self.agent_view_store.focus_session_index(index),
            None => self
                .agent_view_store
                .append_session(SessionContext::new(session.clone())),
        }
    }

    pub(crate) fn handle_attach_to_session(&mut self, session: SessionId) {
        self.focus_transcript_session(&session);
        self.finish_session_attach_wiring(&session, true);
    }

    pub(crate) fn finish_session_attach_wiring(
        &mut self,
        session: &SessionId,
        resume_from_disk: bool,
    ) {
        self.rebuild_focus_occupancy();
        let _ = self.active_session_tx.send(Some(session.clone()));
        if self.navigator.active_view != ViewMode::Agent
            && self.navigator.active_view != ViewMode::Mux
            && self.navigator.agent.resume_view.is_none()
        {
            self.agent_view_store
                .set_navigation_target(Some(session.clone()));
            self.navigator.active_view = ViewMode::Agent;
            self.board_store.menu_focus_to_columns();
        }
        self.spawn_set_active_session(session.clone());
        self.refresh_session_chrome(session.clone());
        self.spawn_hydrate_pending_input(session.clone());
        self.spawn_load_supervisors(session.clone());
        self.spawn_hydrate_debug_state(session.clone());
        if resume_from_disk {
            self.spawn_transcript_resume(session);
        }
    }

    pub(crate) fn handle_reattach_session(&mut self, session: SessionId) {
        self.focus_transcript_session(&session);
        self.finish_session_attach_wiring(&session, false);
        self.spawn_catch_up_from_buffer(&session);
    }

    pub(crate) fn spawn_catch_up_from_buffer(&mut self, session: &SessionId) {
        self.request_transcript(session);
    }

    pub(crate) fn handle_session_resume_complete(&mut self, session: SessionId) {
        self.request_transcript(&session);
    }

    pub(crate) fn spawn_list_sessions(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let project_path = std::env::current_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        let backend = self.backend.clone();
        let sender = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Ok(sessions) = backend.list_sessions(project_path).await {
                let _ = sender.send(Action::SessionListLoaded(sessions));
            }
            super::session_storage::publish_manifest_issues(backend.as_ref(), &sender, false).await;
        }));
    }
}
