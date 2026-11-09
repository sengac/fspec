use super::*;
use codelet_core::persistence::input_receipts::InputOrigin;

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

    /// BUG-554: a submission ends only the draft it was typed from. Clear
    /// the pending-input draft while it is exactly `submitted`; any other
    /// draft is newer (typed while the submission was in flight, or never
    /// submitted) and is kept. Compared and cleared under one write lock.
    pub fn clear_pending_input_if(&self, submitted: &str) {
        let mut draft = self
            .pending_input
            .write()
            .expect("pending_input lock poisoned");
        if draft.as_deref() == Some(submitted) {
            *draft = None;
        }
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
        mut input: IncomingMessage,
    ) -> std::result::Result<(), String> {
        self.ensure_restored()?;
        input.interrupt_epoch = self.queued_input_epoch();
        let origin = InputOrigin::Supervisor {
            role: input.role_name.clone(),
            source_session: input.source_session_id.clone(),
        };
        let submission = self.retain_submission(
            origin,
            input.interrupt_epoch,
            &format_incoming_message(&input),
            input.images.as_deref().unwrap_or_default(),
        )?;
        input.submission_id = Some(submission);
        self.incoming_message_tx.try_send(input).map_err(|e| {
            let reason = format!("Failed to queue supervisor input: {}", e);
            self.refuse_submission(submission, &reason);
            reason
        })?;
        self.incoming_message_pending
            .fetch_add(1, Ordering::Release);
        Ok(())
    }

    pub fn pending_incoming_message_count(&self) -> usize {
        self.incoming_message_pending.load(Ordering::Acquire)
    }

    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String> {
        self.start_turn(input, thinking_config, TurnEcho::Echo)
    }

    /// BUG-348: start steers the running turn never reached as one new
    /// turn. Each steer was echoed when it was submitted, so this turn does
    /// not echo the joined text again.
    pub(super) fn send_steers_as_turn(
        &self,
        input: String,
        thinking_config: Option<String>,
    ) -> Result<(), String> {
        self.start_turn(input, thinking_config, TurnEcho::AlreadyEchoed)
    }

    fn start_turn(
        &self,
        input: String,
        thinking_config: Option<String>,
        echo: TurnEcho,
    ) -> Result<(), String> {
        self.ensure_restored()?;
        let interrupt_epoch = self.compaction_epoch();
        let submission =
            self.retain_submission(InputOrigin::User, Some(interrupt_epoch), &input, &[])?;
        // BUG-554: only the draft this turn was typed from ends; a queued
        // input, joined steers or a loop firing never wipe another draft.
        self.clear_pending_input_if(&input);
        if echo == TurnEcho::Echo {
            self.handle_output(StreamChunk::user_input(input.clone()));
        }
        self.set_status(SessionStatus::Running);
        self.input_tx
            .try_send(PromptInput {
                input,
                thinking_config,
                compaction_epoch: None,
                interrupt_epoch,
                submission: Some(submission),
            })
            .map_err(|e| {
                self.set_status(SessionStatus::Idle);
                let reason = format!("Failed to send input: {}", e);
                self.refuse_submission(submission, &reason);
                reason
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
        // RIG-016: steers the cancelled turn never reached stay parked.
        codelet_common::input_inbox::park_steers(self.id);
        self.is_interrupted.store(true, Ordering::Release);
        self.interrupt_epoch.fetch_add(1, Ordering::AcqRel);
        // BUG-605: answer the question or approval the turn waits on.
        self.cancel_pending_interactions();
        // AMGR-021: what this session holds for its sub-agents moves up.
        self.pass_up_held_requests();
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

/// Whether starting a turn echoes its input as a `UserInput` chunk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TurnEcho {
    /// Echo the input (a fresh submission).
    Echo,
    /// The input was already echoed when it was submitted (BUG-348 steers).
    AlreadyEchoed,
}
