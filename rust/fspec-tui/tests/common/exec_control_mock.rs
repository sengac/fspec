//! TOOL-027: MockBackend recording for `control_exec_stdin`.

use parking_lot::Mutex;

use codelet_rpc_types::{ExecStdinControl, SessionId};

use super::MockBackend;

/// Every `control_exec_stdin` call, and an optional scripted failure.
#[derive(Default)]
pub struct ExecControlMock {
    calls: Mutex<Vec<(SessionId, String, ExecStdinControl)>>,
    error: Mutex<Option<String>>,
}

impl ExecControlMock {
    /// Record a call and return the scripted error, if any.
    pub fn record(&self, call: (SessionId, String, ExecStdinControl)) -> Result<(), String> {
        self.calls.lock().push(call);
        match self.error.lock().clone() {
            Some(message) => Err(message),
            None => Ok(()),
        }
    }

    /// Every `(agent session, exec session, control)` in call order.
    pub fn calls(&self) -> Result<Vec<(SessionId, String, ExecStdinControl)>, String> {
        Ok(self.calls.lock().clone())
    }

    /// Make every later call fail with `message`.
    pub fn fail_with(&self, message: &str) -> Result<(), String> {
        *self.error.lock() = Some(message.to_string());
        Ok(())
    }
}

impl MockBackend {
    /// TOOL-027: the `control_exec_stdin` recorder.
    pub fn exec_control(&self) -> &ExecControlMock {
        &self.exec_control
    }
}
