//! BUG-576: every `set_session_model` call in order, and scripted failures
//! for chosen models.

use parking_lot::Mutex;
use std::collections::HashMap;

use codelet_rpc_types::SessionId;

#[derive(Default)]
pub struct ModelSelectionMock {
    failures: Mutex<HashMap<String, String>>,
    calls: Mutex<Vec<(SessionId, String, String)>>,
}

impl ModelSelectionMock {
    /// Make every later `set_session_model` for `model` fail with `message`.
    pub fn fail_model(&self, model: &str, message: &str) {
        self.failures
            .lock()
            .insert(model.to_string(), message.to_string());
    }

    /// Every `(session, provider, model)` passed to `set_session_model`.
    pub fn calls(&self) -> Vec<(SessionId, String, String)> {
        self.calls.lock().clone()
    }

    /// Record a call; the scripted failure for its model, if any.
    pub(super) fn record(
        &self,
        session: &SessionId,
        provider: &str,
        model: &str,
    ) -> Option<String> {
        self.calls
            .lock()
            .push((session.clone(), provider.to_string(), model.to_string()));
        self.failures.lock().get(model).cloned()
    }
}
