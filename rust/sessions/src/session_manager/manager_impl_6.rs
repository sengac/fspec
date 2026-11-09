//! SessionManager inherent methods (part 6).

use super::*;

impl SessionManager {
    /// BUG-328: the live session input is sent to. A session this process
    /// does not hold is refused — clients resume it explicitly with
    /// `resume_session`; input never resumes it implicitly.
    pub(crate) fn live_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Arc<BackgroundSession>, String> {
        self.get_session(&session_id.value).map_err(|_| {
            tracing::warn!(session_id = %session_id.value, "input refused: session is not live");
            codelet_core::session_manager_handle::session_not_live(session_id)
        })
    }

    /// Destroy a session
    pub fn destroy_session(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        tracing::info!(
            session_id = %uuid,
            "destroy_session: starting session destruction"
        );

        self.chain_of_command.cleanup_subordinate(uuid);
        self.chain_of_command.remove_supervisor(uuid);

        let session = self
            .sessions
            .write()
            .expect("sessions lock poisoned")
            .shift_remove(&uuid);

        // BUG-206: a closed session must never be referenced by the
        // active-session pointer. The pointer is in-memory and sticky
        // (BackToBoard does not clear it — user decision 2026-10-10),
        // so without this guard the last-viewed-but-CLOSED session
        // would keep winning the auto-restore candidate resolution
        // until the next client start. Clear it when it pointed at
        // the destroyed session (parity with
        // StubSessionManagerHandle::destroy_session).
        {
            let mut active = self
                .active_session_id
                .write()
                .expect("active_session lock poisoned");
            if *active == Some(uuid) {
                *active = None;
            }
        }

        tracing::info!(
            session_id = %uuid,
            session_removed = session.is_some(),
            "destroy_session: shift_remove completed"
        );

        if let Some(session) = session {
            tracing::info!(
                session_id = %uuid,
                "destroy_session: session removed from in-memory map"
            );
            // RPC-433: fire the removal broadcast AFTER the map removal so
            // the event reflects a state that already holds. Only a live
            // destroy reaches this block (an unknown id falls through to
            // the Err arm), so the event is exactly-once per live session
            // (rule R2). The id is the SessionId newtype, matching what a
            // connected client's tabs key on.
            let _ = self
                .session_removed_tx
                .send(SessionId::from(uuid.to_string()));
            session.interrupt();
            // BUG-288: release the manifest writer lease so another
            // process can delete or reopen the destroyed session.
            codelet_core::persistence::release_session_ownership(uuid);
            codelet_tools::tool_pause::set_active_tool_call(uuid, None);
            self.hooks().stop_footer_poller(id);
            self.hooks().cleanup_session_loops(uuid);
            codelet_tools::cleanup_mcp_session(uuid);
            codelet_tools::tool_admission::unregister_session(uuid);
            unregister_pre_tool_hook(uuid);
            codelet_tools::unregister_bash_abort_flag(uuid);
            // BUG-606: a closed session's "approve for session" choices go with it.
            codelet_tools::blocklist::forget_session_allowances(uuid);
            // BUG-601: the manifest keeps its turned-off rules for a resume.
            codelet_tools::blocklist::forget_session_disabled_rules(uuid);
            codelet_tools::unregister_footer_cwd(uuid);
            codelet_tools::model_capabilities::clear_session_model_vision(uuid);
            // PROV-144: clear the image-budget entry alongside the vision entry.
            codelet_tools::model_capabilities::clear_session_model_max_images(uuid);
            codelet_tools::broadcast_metadata_update();

            // WT-005: terminate-on-close. When the closed session was
            // isolated, flip its git-session manifest (~/.fspec/git-sessions)
            // to terminated=true so `codelet_git::is_orphaned` reports it as
            // prunable and `prune_orphaned_worktrees` can reclaim the
            // worktree. Best-effort: a missing manifest (worktree already
            // merged/discarded — those paths delete it) is a silent no-op,
            // and a failure (e.g. no home dir) is logged + swallowed so the
            // close never fails. Non-isolated sessions skip entirely.
            if session.worktree_path().is_some() {
                match codelet_git::terminate_session(id) {
                    Ok(()) => tracing::info!(
                        session_id = %uuid,
                        "destroy_session: git-session manifest marked terminated (worktree now prunable)"
                    ),
                    Err(e) => tracing::warn!(
                        session_id = %uuid,
                        error = %e,
                        "destroy_session: failed to mark git-session manifest terminated (best-effort; close unaffected)"
                    ),
                }
            }

            // PARITY FIX: Do NOT delete the session manifest from disk.
            // The TypeScript reference implementation's "Close Session" (exit dialog)
            // calls sessionManagerDestroy() which only kills the in-memory session.
            // The manifest persists on disk so the user can resume later via /resume.
            //
            // Manifest deletion is a separate operation: "Delete This Session" from
            // the resume view calls persistenceDeleteSession() which maps to
            // backend.persistence_delete_session() → codelet_core::persistence::delete_session().
            //
            // See: src/tui/services/sessionService.ts destroySession() vs persistenceDeleteSession()

            tracing::info!(
                session_id = %uuid,
                "destroy_session: session destruction complete"
            );

            Ok(())
        } else {
            tracing::warn!(
                session_id = %uuid,
                "destroy_session: session not found in memory"
            );
            Err(format!("Session not found: {}", id))
        }
    }
    /// RPC-436: clear a session's history through the manager, firing the
    /// session-cleared broadcast.
    ///
    /// Liveness gate (rule R2 of session-events-daemon-cleared-broadcast):
    /// the event fires ONLY when the clear actually succeeded — an unknown
    /// id returns the pre-existing `Err("Session not found: …")` contract
    /// and emits nothing, mirroring `destroy_session`'s removed-broadcast
    /// idempotency (rule R2 of session-events-daemon-fanout.feature).
    ///
    /// The clear itself runs through `BackgroundSession::clear_history`
    /// (the documented single source of truth for clear, TUI-065), which
    /// fires the event on this manager's `session_cleared_tx` — the sender
    /// is injected at session creation (RPC-041 idiom, like `chunks_tx`),
    /// so a session created by THIS manager always fires on this manager's
    /// broadcast.
    /// `block_in_place` mirrors the RPC-073 contract for the handle path
    /// (`clear_history` takes `self.inner.blocking_lock()`).
    pub fn clear_session_history(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        tracing::debug!(session_id = %uuid, "clear_session_history: starting");

        let session = self.get_session(id)?;
        tokio::task::block_in_place(|| session.clear_history()).map_err(|error| error.to_string())
    }
    // === ChainOfCommand delegation methods (WATCH-002) ===

    pub fn add_supervisor(
        &self,
        subordinate_id: Uuid,
        supervisor_id: Uuid,
    ) -> std::result::Result<(), String> {
        self.chain_of_command
            .add_supervisor(subordinate_id, supervisor_id)
    }
    pub fn remove_supervisor(&self, supervisor_id: Uuid) {
        self.chain_of_command.remove_supervisor(supervisor_id)
    }
    pub fn get_supervisors(&self, subordinate_id: Uuid) -> Vec<Uuid> {
        self.chain_of_command.get_supervisors(subordinate_id)
    }
    pub fn get_subordinate(&self, supervisor_id: Uuid) -> Option<Uuid> {
        self.chain_of_command.get_subordinate(supervisor_id)
    }
    pub fn get_subordinates(&self, supervisor_id: Uuid) -> Vec<Uuid> {
        self.chain_of_command.get_subordinates(supervisor_id)
    }
}
