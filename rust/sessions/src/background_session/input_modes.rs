//! RIG-016: steer, queue and interrupt input submitted to a busy session.

use super::*;
use codelet_common::input_inbox::{self, InboxEntry};
use codelet_rpc_types::InputMode;

impl BackgroundSession {
    fn turn_running(&self) -> bool {
        matches!(
            self.get_status(),
            SessionStatus::Running | SessionStatus::Compacting
        )
    }

    /// Submit input under `mode`. An idle session starts it as a new turn
    /// (a queue entry first lets any parked inputs ahead of it run); a busy
    /// one steers it into the running turn, queues it behind that turn, or
    /// interrupts the turn and parks it.
    pub fn submit_input(
        &self,
        text: String,
        thinking_config: Option<String>,
        mode: InputMode,
    ) -> Result<(), String> {
        let entry = InboxEntry {
            text,
            thinking_config,
        };
        match (self.turn_running(), mode) {
            (false, InputMode::Queue) => {
                self.clear_pending_input_if(&entry.text); // BUG-554
                input_inbox::push_queued(self.id, entry);
                self.resume_queued().map(|_| ())
            }
            (false, InputMode::Steer | InputMode::Interrupt) => {
                self.send_input(entry.text, entry.thinking_config)
            }
            (true, InputMode::Steer) => {
                self.clear_pending_input_if(&entry.text); // BUG-554
                self.handle_output(StreamChunk::user_input(entry.text.clone()));
                input_inbox::push_steer(self.id, entry);
                Ok(())
            }
            (true, InputMode::Queue) => {
                self.clear_pending_input_if(&entry.text); // BUG-554
                input_inbox::push_queued(self.id, entry);
                Ok(())
            }
            (true, InputMode::Interrupt) => {
                self.clear_pending_input_if(&entry.text); // BUG-554
                input_inbox::push_queued_front(self.id, entry);
                self.interrupt();
                Ok(())
            }
        }
    }

    /// Inputs parked or queued behind the running turn, oldest first.
    pub fn queued_inputs(&self) -> Vec<String> {
        input_inbox::queued_texts(self.id)
    }

    /// Start the next parked input as a new turn. `Ok(false)` when nothing
    /// is parked or a turn is still running.
    pub fn resume_queued(&self) -> Result<bool, String> {
        if self.turn_running() {
            return Ok(false);
        }
        match input_inbox::pop_queued(self.id) {
            Some(entry) => self
                .send_input(entry.text, entry.thinking_config)
                .map(|()| true),
            None => Ok(false),
        }
    }

    /// After a turn that completed without an interrupt: start steers it
    /// never reached as one new turn, otherwise the next queued input.
    pub fn continue_with_inbox(&self) -> Result<bool, String> {
        let steers = input_inbox::take_steers(self.id);
        if steers.is_empty() {
            return self.resume_queued();
        }
        let thinking_config = steers.iter().rev().find_map(|e| e.thinking_config.clone());
        let text = steers
            .into_iter()
            .map(|entry| entry.text)
            .collect::<Vec<_>>()
            .join("\n\n");
        self.send_steers_as_turn(text, thinking_config)
            .map(|()| true)
    }
}
