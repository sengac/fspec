use crate::views::ViewMode;

use super::state::App;

impl App {
    pub(crate) fn handle_enter_work_unit(&mut self, id: &str) {
        let associated = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|session| {
                self.agent_view_store
                    .work_unit_context_for(&session.id)
                    .is_some_and(|context| context.id == id)
            });
        if let Some(index) = associated {
            let session = self.agent_view_store.open_sessions()[index].id.clone();
            self.switch_to_session_index(index);
            self.board_store.menu_focus_to_columns();
            if self.navigator.active_view == ViewMode::Mux {
                self.mux_sync_window();
                self.navigator.mux.focus_agent_pane_for_session(&session);
            } else {
                self.navigator.active_view = ViewMode::Agent;
            }
        } else if let Some(context) = self.lookup_work_unit_context(id) {
            self.open_creation_dialog(None, Some(context));
        }
    }
}
