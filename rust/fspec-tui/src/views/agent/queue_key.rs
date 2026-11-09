//! RIG-016: Alt+Enter queues the composer text while the focused session's
//! turn is running, and (TUI-115) while it is compacting. Idle, Alt+Enter
//! keeps its legacy newline binding.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::EventResult;

use super::AgentView;

impl AgentView {
    /// Submit the buffer as a queued input on Alt+Enter while busy.
    pub(super) fn queue_on_alt_enter(&mut self, key: &KeyEvent) -> Option<EventResult> {
        let busy = self.last_is_running || self.last_is_compacting;
        if !busy || key.code != KeyCode::Enter || key.modifiers != KeyModifiers::ALT {
            return None;
        }
        let value = self.input.value();
        if value.is_empty() {
            return Some(EventResult::ignored());
        }
        self.input.reset();
        self.emit_submission(value, true);
        Some(EventResult::consumed())
    }
}
