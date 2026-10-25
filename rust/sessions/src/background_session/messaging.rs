use super::*;

impl BackgroundSession {
    pub fn get_pending_input(&self) -> Option<String> {
        self.pending_input
            .read()
            .expect("pending_input lock poisoned")
            .clone()
    }

    pub fn set_pending_input(&self, input: Option<String>) {
        *self
            .pending_input
            .write()
            .expect("pending_input lock poisoned") = input;
    }

    pub fn set_role(&self, role: String) {
        *self.role.write().expect("role lock poisoned") = Some(role);
    }

    pub fn get_role(&self) -> Option<String> {
        self.role.read().expect("role lock poisoned").clone()
    }

    pub fn clear_role(&self) {
        *self.role.write().expect("role lock poisoned") = None;
    }

    pub fn receive_incoming_message(
        &self,
        input: IncomingMessage,
    ) -> std::result::Result<(), String> {
        self.incoming_message_tx
            .try_send(input)
            .map_err(|e| format!("Failed to queue supervisor input: {}", e))?;
        self.incoming_message_pending
            .fetch_add(1, Ordering::Release);
        Ok(())
    }

    pub fn pending_incoming_message_count(&self) -> usize {
        self.incoming_message_pending.load(Ordering::Acquire)
    }

    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String> {
        self.set_pending_input(None);
        self.handle_output(StreamChunk::user_input(input.clone()));
        self.set_status(SessionStatus::Running);
        self.reset_interrupt();
        self.input_tx
            .try_send(PromptInput {
                input,
                thinking_config,
                compaction_epoch: None,
            })
            .map_err(|e| {
                self.set_status(SessionStatus::Idle);
                format!("Failed to send input: {}", e)
            })
    }

    pub fn interrupt(&self) {
        self.clear_provider_presentation();
        let mut reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *reason = Some(codelet_rpc_types::InterruptionReason::User);
        let _ = self.take_pending_loop_abort_note();
        self.is_interrupted.store(true, Ordering::Release);
        self.interrupt_epoch.fetch_add(1, Ordering::AcqRel);
        request_bash_abort(self.id);
        self.interrupt_notify.notify_one();
    }

    pub fn reset_interrupt(&self) {
        let mut reason = self
            .interruption_reason
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *reason = None;
        let _ = self.take_pending_loop_abort_note();
        self.is_interrupted.store(false, Ordering::Release);
        clear_bash_abort(self.id);
    }

    pub fn get_interrupt_notify(&self) -> Arc<Notify> {
        self.interrupt_notify.clone()
    }

    pub fn set_pending_loop_abort_note(&self, note: String) {
        if let Ok(mut guard) = self.pending_loop_abort_note.lock() {
            *guard = Some(note);
        }
    }

    pub fn take_pending_loop_abort_note(&self) -> Option<String> {
        self.pending_loop_abort_note
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
    }

    pub fn has_pending_loop_abort_note(&self) -> bool {
        self.pending_loop_abort_note
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}
