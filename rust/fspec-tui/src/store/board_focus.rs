use codelet_rpc_types::SessionId;

use super::BoardStore;

impl BoardStore {
    pub(crate) fn replace_focus_occupancy(&mut self, bindings: Vec<(String, SessionId)>) {
        self.session_attachments.clear();
        for (card, session) in bindings {
            self.session_attachments.entry(card).or_insert(session);
        }
    }
}
