macro_rules! compose_handle_impl_3 {
    ($($collected:tt)*) => {
        compose_handle_impl_4! {
            $($collected)*
    /// BUG-206: the live in-memory session IDs — the auto-restore
    /// liveness gate. In-memory map ONLY: closed / manifest-only
    /// sessions never appear here.
    fn list_live_sessions(&self) -> Result<Vec<SessionId>, String> {
        Ok(SessionManager::all_in_memory_session_ids(self)
            .into_iter()
            .map(SessionId::new)
            .collect())
    }
    fn clear_active_session(&self) -> Result<(), String> {
        SessionManager::clear_active_session(self);
        Ok(())
    }
    fn get_active_session(&self) -> Result<Option<SessionId>, String> {
        Ok(SessionManager::get_active_session(self).map(|u| SessionId::new(u.to_string())))
    }
    /// BUG-437: an unknown session is an error, not the start workspace.
    fn get_effective_cwd(&self, session_id: &SessionId) -> Result<PathBuf, String> {
        Ok(self.get_session(&session_id.value)?.effective_cwd())
    }
    fn get_supervisors(&self, session_id: &SessionId) -> Result<Vec<SessionId>, String> {
        // BUG-464: an invalid id or an unknown session is the lookup error,
        // not "no supervisors".
        let uuid = self.get_session(&session_id.value)?.id;
        Ok(SessionManager::get_supervisors(self, uuid)
            .into_iter()
            .map(|u| SessionId::new(u.to_string()))
            .collect())
    }
    /// RPC-061: delegate to ChainOfCommand::add_supervisor (surfaces
    /// the "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" errors verbatim).
    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        let sub_uuid = parse_session_uuid(subordinate_id)?;
        let sup_uuid = parse_session_uuid(supervisor_id)?;
        SessionManager::add_supervisor(self, sub_uuid, sup_uuid)
    }
    /// RPC-061: delegate to ChainOfCommand::remove_supervisor. The
    /// underlying method is `()`-returning; we wrap into Ok(()).
    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        let sup_uuid = parse_session_uuid(supervisor_id)?;
        SessionManager::remove_supervisor(self, sup_uuid);
        Ok(())
    }
    /// RPC-061: first subordinate of the supervisor (or None).
    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        let sup_uuid = logged_session_uuid(supervisor_id)?;
        SessionManager::get_subordinate(self, sup_uuid).map(|u| SessionId::new(u.to_string()))
    }
    /// RPC-061: every subordinate of the supervisor.
    fn get_subordinates(&self, supervisor_id: &SessionId) -> Result<Vec<SessionId>, String> {
        // BUG-465: an invalid id or an unknown session is the lookup error.
        let sup_uuid = self.get_session(&supervisor_id.value)?.id;
        Ok(SessionManager::get_subordinates(self, sup_uuid)
            .into_iter()
            .map(|u| SessionId::new(u.to_string()))
            .collect())
    }
    /// RPC-061: queue a supervisor message onto the subordinate's
    /// BackgroundSession `receive_incoming_message` mpsc channel.
    /// Returns Err("Session not found: …") when the subordinate id
    /// does not match a live session.
    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        let session = self.get_session(&subordinate_id.value)?;
        let bridge_images = message.images.map(|imgs| {
            imgs.into_iter()
                .map(|img| crate::background_session::BridgeImageData {
                    data: img.data,
                    media_type: img.media_type,
                })
                .collect::<Vec<_>>()
        });
        let input = crate::background_session::IncomingMessage::with_images(
            message.source_session_id,
            message.role_name,
            message.message,
            bridge_images,
        )?;
        session.receive_incoming_message(input)
    }
    fn get_debug_enabled(&self, session_id: &SessionId) -> Result<bool, String> {
        Ok(self.get_session(&session_id.value)?.get_debug_enabled())
    }
    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) -> Result<(), String> {
        // BUG-464: an unknown session is the lookup error, not a no-op.
        self.get_session(&session_id.value)?.set_debug_enabled(enabled);
        Ok(())
    }
    fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                // RPC-055: port of NAPI's session_toggle_debug
                // (rust/napi/src/session_bindings.rs:2645). We:
                //   1. Compute per-session debug directory
                //      `{debug_dir}/debug/{session_id}/`.
                //   2. Lock the session's DebugCaptureManager and call
                //      start_capture or stop_capture depending on the
                //      current enabled flag.
                //   3. Persist the new enabled flag on the
                //      BackgroundSession atomic.
                //   4. Emit `StreamChunk::DebugStateChange` for the TUI.
                let session_debug_dir = std::path::PathBuf::from(debug_dir)
                    .join("debug")
                    .join(session.id.to_string());
                let was_enabled = session.get_debug_enabled();
                let result = {
                    let mut manager = session.debug_capture.lock().map_err(|_| {
                        "Failed to acquire lock on per-session debug capture manager".to_string()
                    })?;
                    manager.set_debug_directory_raw(session_debug_dir);
                    if was_enabled {
                        manager.stop_capture().map_err(|e| e.to_string())?
                    } else {
                        manager.start_capture().map_err(|e| e.to_string())?
                    }
                };
                let new_enabled = !was_enabled;
                session.set_debug_enabled(new_enabled);
                let _ = SessionManager::chunks_tx(self).send((
                    session_id.clone(),
                    StreamChunk::debug_state_change(new_enabled),
                ));
                Ok(result)
            }
            Err(error) => Err(error),
        }
    }
    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        // RPC-055: pre-session global toggle path. Delegates to the
        // codelet-common global DebugCaptureManager singleton. This is
        // ONLY used before any session exists — once a session is
        // created, `toggle_debug(session_id, debug_dir)` is the
        // preferred entry point (it uses the per-session manager on
        // BackgroundSession).
        match codelet_common::debug_capture::get_debug_capture_manager() {
            Ok(arc) => {
                let mut manager = arc.lock().map_err(|_| {
                    "Failed to acquire lock on global debug capture manager".to_string()
                })?;
                manager.set_debug_directory(path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to access debug capture manager: {e}")),
        }
    }
    // AMGR-020: every reply names the request id and is the user's.
    fn pause_human_action(
        &self,
        session_id: &SessionId,
        request_id: &str,
        reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<(), String> {
        self.get_session(&session_id.value)?.answer_pause(
            request_id,
            &codelet_rpc_types::RequestHolder::User,
            crate::background_session::PauseReply::HumanAction(reply),
        )
    }
    fn pause_confirm(
        &self,
        session_id: &SessionId,
        request_id: &str,
        accept: bool,
    ) -> Result<(), String> {
        self.get_session(&session_id.value)?.answer_pause(
            request_id,
            &codelet_rpc_types::RequestHolder::User,
            crate::background_session::PauseReply::Confirm { accept },
        )
    }
    fn pause_triple(
        &self,
        session_id: &SessionId,
        request_id: &str,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        self.get_session(&session_id.value)?.answer_pause(
            request_id,
            &codelet_rpc_types::RequestHolder::User,
            crate::background_session::PauseReply::Triple(choice),
        )
    }
        }
    };
}
