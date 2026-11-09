macro_rules! compose_stub_handle_2 {
    ($($collected:tt)*) => {
        compose_stub_handle_3! {
            $($collected)*
    fn restore_session_messages(
        &self,
        _session_id: &SessionId,
        _envelopes: Vec<String>,
    ) -> Result<(), String> {
        Ok(())
    }
    fn restore_session_token_state(
        &self,
        _session_id: &SessionId,
        _state: TokenRestoreState,
    ) -> Result<(), String> {
        Ok(())
    }
    /// RPC-049: deterministic stub override — increments the per-stub
    /// call counter and returns `Ok(())` WITHOUT reaching for the
    /// on-disk persistence layer (the default trait impl would call
    /// `codelet_core::persistence::load_session` which doesn't make
    /// sense against the stub's in-memory session table).
    fn resume_session(&self, _session_id: &SessionId) -> Result<(), String> {
        self.resume_session_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn get_work_unit_context(&self, session_id: &SessionId) -> Result<Option<WorkUnitContext>, String> {
        Ok({
        self.get_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let guard = stub_lock(&self.work_unit_ctx, "work_unit_ctx");
            guard.get(session_id).cloned()
        }
    })
    }
    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        self.set_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let mut guard = stub_lock(&self.work_unit_ctx, "work_unit_ctx");
            match ctx {
                Some(c) => {
                    guard.insert(session_id.clone(), c);
                }
                None => {
                    guard.remove(session_id);
                }
            }
        }
        Ok(())
    }
    fn get_pending_input(&self, session_id: &SessionId) -> Result<Option<String>, String> {
        Ok({
        {
            let guard = stub_lock(&self.pending_input, "pending_input");
            guard.get(session_id).cloned()
        }
    })
    }
    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.pending_input, "pending_input");
            match text {
                Some(t) => {
                    guard.insert(session_id.clone(), t);
                }
                None => {
                    guard.remove(session_id);
                }
            }
        }
        Ok(())
    }
    fn set_active_session(&self, session_id: &SessionId) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.active_session, "active_session");
            *guard = Some(session_id.clone());
        }
        Ok(())
    }
    fn clear_active_session(&self) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.active_session, "active_session");
            *guard = None;
        }
        Ok(())
    }
    fn get_active_session(&self) -> Result<Option<SessionId>, String> {
        Ok({
        stub_lock(&self.active_session, "active_session").clone()
    })
    }
    fn get_effective_cwd(&self, session_id: &SessionId) -> Result<PathBuf, String> {
        // BUG-541: a directory given with set_effective_cwd, else an error.
        stub_lock(&self.effective_cwds, "effective_cwds")
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "the stub session manager has no working directory for session {}",
                    session_id.value
                )
            })
    }
    fn get_supervisors(&self, session_id: &SessionId) -> Result<Vec<SessionId>, String> {
        Ok({
        self.get_supervisors_calls.fetch_add(1, Ordering::SeqCst);
        stub_lock(&self.sub_to_sup, "sub_to_sup")
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    })
    }
    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        self.add_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        let mut sup2subs = self
            .sup_to_subs
            .lock()
            .map_err(|_| "sup_to_subs lock poisoned".to_string())?;
        // Duplicate detection.
        if let Some(existing) = sup2subs.get(supervisor_id) {
            if existing.iter().any(|s| s == subordinate_id) {
                return Err("subordinate already registered under this supervisor".to_string());
            }
        }
        // BFS cycle detection mirroring
        // `codelet_sessions::ChainOfCommand::add_supervisor`.
        {
            let mut visited: std::collections::HashSet<SessionId> =
                std::collections::HashSet::new();
            let mut queue: std::collections::VecDeque<SessionId> =
                std::collections::VecDeque::new();
            queue.push_back(subordinate_id.clone());
            visited.insert(subordinate_id.clone());
            while let Some(current) = queue.pop_front() {
                if let Some(subs) = sup2subs.get(&current) {
                    for s in subs.iter() {
                        if s == supervisor_id {
                            return Err("circular supervision not allowed".to_string());
                        }
                        if visited.insert(s.clone()) {
                            queue.push_back(s.clone());
                        }
                    }
                }
            }
        }
        sup2subs
            .entry(supervisor_id.clone())
            .or_default()
            .push(subordinate_id.clone());
        drop(sup2subs);
        let mut sub2sup = self
            .sub_to_sup
            .lock()
            .map_err(|_| "sub_to_sup lock poisoned".to_string())?;
        sub2sup
            .entry(subordinate_id.clone())
            .or_default()
            .push(supervisor_id.clone());
        Ok(())
    }
    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        self.remove_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        let subordinate_ids = {
            let mut sup2subs = self
                .sup_to_subs
                .lock()
                .map_err(|_| "sup_to_subs lock poisoned".to_string())?;
            sup2subs.remove(supervisor_id).unwrap_or_default()
        };
        if !subordinate_ids.is_empty() {
            let mut sub2sup = self
                .sub_to_sup
                .lock()
                .map_err(|_| "sub_to_sup lock poisoned".to_string())?;
            for sid in subordinate_ids {
                if let Some(supervisors) = sub2sup.get_mut(&sid) {
                    supervisors.retain(|s| s != supervisor_id);
                    if supervisors.is_empty() {
                        sub2sup.remove(&sid);
                    }
                }
            }
        }
        Ok(())
    }
    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        self.get_subordinate_calls.fetch_add(1, Ordering::SeqCst);
        stub_lock(&self.sup_to_subs, "sup_to_subs")
            .get(supervisor_id)
            .and_then(|v| v.first().cloned())
    }
    fn get_subordinates(&self, supervisor_id: &SessionId) -> Result<Vec<SessionId>, String> {
        Ok({
        self.get_subordinates_calls.fetch_add(1, Ordering::SeqCst);
        stub_lock(&self.sup_to_subs, "sup_to_subs")
            .get(supervisor_id)
            .cloned()
            .unwrap_or_default()
    })
    }
    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        self.receive_incoming_message_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let mut guard = stub_lock(&self.recorded_incoming_messages, "recorded_incoming_messages");
            guard.push((subordinate_id.clone(), message));
        }
        Ok(())
    }
    fn get_debug_enabled(&self, session_id: &SessionId) -> Result<bool, String> {
        Ok({
        {
            let guard = stub_lock(&self.debug_enabled, "debug_enabled");
            guard.get(session_id).copied().unwrap_or(false)
        }
    })
    }
    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.debug_enabled, "debug_enabled");
            guard.insert(session_id.clone(), enabled);
        }
        Ok(())
    }
    fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String> {
        self.toggle_debug_calls.fetch_add(1, Ordering::SeqCst);
        let current = self.get_debug_enabled(session_id)?;
        let next = !current;
        self.set_debug_enabled(session_id, next)?;
        let _ = self
            .chunks_tx
            .send((session_id.clone(), StreamChunk::debug_state_change(next)));
        Ok(debug_dir.to_string())
    }
    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        self.set_debug_directory_calls
            .fetch_add(1, Ordering::SeqCst);
        let _ = path;
        Ok(())
    }
    // BUG-631 / AMGR-023: each reply answers only the pause kind it is
    // for, as the daemon checks it: pause_human_action a human action,
    // confirm a two-choice confirm, triple a three-choice.
    fn pause_human_action(
        &self,
        session_id: &SessionId,
        request_id: &str,
        _reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<(), String> {
        self.answer_stub_pause(session_id, request_id, "pause_human_action", |pending| {
            pending.payload.kind == codelet_rpc_types::PauseKind::HumanAction
        })
    }
    fn pause_confirm(
        &self,
        session_id: &SessionId,
        request_id: &str,
        _accept: bool,
    ) -> Result<(), String> {
        self.answer_stub_pause(session_id, request_id, "pause_confirm", |pending| {
            pending.payload.kind == codelet_rpc_types::PauseKind::Confirm
        })
    }
    fn pause_triple(
        &self,
        session_id: &SessionId,
        request_id: &str,
        _choice: ApprovalChoice,
    ) -> Result<(), String> {
        self.answer_stub_pause(session_id, request_id, "pause_triple", |pending| {
            pending.payload.kind == codelet_rpc_types::PauseKind::Triple
        })
    }
        }
    };
}
