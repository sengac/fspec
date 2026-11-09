//! BUG-325 / BUG-328 / RPC-450: scripted outcomes of the mock's custom-model
//! writes and `send_input`, and the card requests it received.

use super::MockBackend;

impl MockBackend {
    /// Make every add/update/delete_custom_model call fail with `message`.
    pub fn fail_custom_model_writes(&self, message: &str) {
        *self.orchestration.custom_model_write_error.lock() = Some(message.to_string());
    }

    pub(super) fn custom_model_write_result(&self) -> anyhow::Result<()> {
        match self.orchestration.custom_model_write_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    /// BUG-328: make `send_input` refuse with `message`.
    pub fn fail_send_input(&self, message: &str) {
        *self.orchestration.send_input_error.lock() = Some(message.to_string());
    }

    pub(super) fn send_input_result(&self) -> anyhow::Result<()> {
        match self.orchestration.send_input_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    /// RPC-450: record a card details / move request.
    pub(super) fn record_card_request(&self, kind: &str, workspace: &str, id: &str) {
        self.orchestration.card_requests.lock().push((
            kind.to_string(),
            workspace.to_string(),
            id.to_string(),
        ));
    }

    /// RPC-450: every card details / move request, in call order.
    pub fn card_requests(&self) -> Vec<(String, String, String)> {
        self.orchestration.card_requests.lock().clone()
    }
}
