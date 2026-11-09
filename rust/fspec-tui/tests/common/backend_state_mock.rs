//! BUG-466: state the MockBackend stores for calls whose FspecBackend
//! defaults refuse, plus a scripted session-listing failure.

use parking_lot::Mutex;

pub struct BackendState {
    pub continue_state: Mutex<(bool, u32)>,
    pub goal_state: Mutex<Option<(String, Option<String>)>>,
    pub list_sessions_error: Mutex<Option<String>>,
}

impl Default for BackendState {
    fn default() -> Self {
        Self {
            continue_state: Mutex::new((false, 10)),
            goal_state: Mutex::new(None),
            list_sessions_error: Mutex::new(None),
        }
    }
}
