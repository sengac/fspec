//! TUI-124 — closing the focused agent or bot: gather what the close will
//! do, show it, then close in order and fold each success.
//!
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature
//!
//! ```text
//! [ Close ] / Esc level 7 ─► request_close(session)
//!     spawn get_subordinates + get_supervisors ─► Action::CloseFactsLoaded
//! CloseFactsLoaded ─► Err  → "The <kind> was not closed: …" reported, no dialog
//!                  └► Ok   → CloseFacts (store: status, isolation, kinds)
//!                            → ExitConfirmationDialog + pending_close
//! AgentExitChoice::CloseSession ─► close task (dispatch_agent_exit.rs):
//!     a bot's sub-agents one by one, then the session; the first failure
//!     stops it ─► Action::CloseFinished { closed, failure }
//! CloseFinished ─► every closed session folds through the session-removed
//!     cleanup (tab, work-unit links, mux window); the closed session also
//!     drops its work unit and returns to the board; a failure is reported.
//! ```
//!
//! Each session is either fully closed or fully open (product owner rule,
//! 2026-11-07): a tab is removed only after its backend close succeeded.

use codelet_rpc_types::{SessionId, SessionStatus};

use crate::components::exit_confirmation_dialog::{
    CloseFacts, CloseRelations, ClosingSubAgent, ExitConfirmationDialog, SessionKind,
    SubAgentStatus, EXIT_CONFIRMATION_DIALOG_ID,
};
use crate::components::Action;

use super::state::App;

impl App {
    /// The kind the dialogs name for `session` (product owner, 2026-11-07).
    pub(crate) fn session_kind(&self, session: &SessionId) -> SessionKind {
        if self.agent_view_store.is_bot_session(session) {
            SessionKind::Bot
        } else {
            SessionKind::Agent
        }
    }

    fn is_working(&self, session: &SessionId) -> bool {
        matches!(
            self.agent_view_store.session_status_for(session).copied(),
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting),
        )
    }

    fn is_isolated(&self, session: &SessionId) -> bool {
        self.agent_view_store
            .isolation_state_for(session)
            .is_some_and(|state| state.is_isolated)
    }

    /// The [ Close ] button and Esc level 7: fetch the focused session's
    /// relations; the dialog opens when they arrive. No-op while a close
    /// dialog is open or without a runtime to fetch on.
    pub(crate) fn request_close(&mut self, session: SessionId) {
        if self.compositor.contains(EXIT_CONFIRMATION_DIALOG_ID) {
            return;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let kind = self.session_kind(&session);
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let result = async {
                let sub_agents = backend
                    .get_subordinates(session.clone())
                    .await
                    .map_err(|error| format!("its sub-agents could not be listed: {error:#}"))?;
                let started_by = backend
                    .get_supervisors(session.clone())
                    .await
                    .map_err(|error| format!("what started it could not be listed: {error:#}"))?;
                Ok(CloseRelations {
                    sub_agents,
                    started_by,
                })
            }
            .await;
            let _ = action_tx.send(Action::CloseFactsLoaded {
                session,
                kind,
                result,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Open the close dialog for `session` with what closing will do, or
    /// report why nothing was closed.
    pub(crate) fn handle_close_facts_loaded(
        &mut self,
        session: SessionId,
        kind: SessionKind,
        result: Result<CloseRelations, String>,
    ) {
        let relations = match result {
            Ok(relations) => relations,
            Err(error) => {
                let message = format!("The {} was not closed: {error}", kind.word());
                let _ = self.action_tx.send(Action::SessionOperationFailed(message));
                return;
            }
        };
        // The user moved on (another session focused, or a dialog already
        // open): open nothing rather than a dialog about another session.
        if self.agent_view_store.current_session() != Some(&session)
            || self.compositor.contains(EXIT_CONFIRMATION_DIALOG_ID)
        {
            return;
        }
        let mut facts = CloseFacts::new(session.clone(), kind);
        facts.busy = self.is_working(&session);
        facts.isolated = self.is_isolated(&session);
        match kind {
            SessionKind::Bot => {
                facts.closing_sub_agents = relations
                    .sub_agents
                    .iter()
                    .map(|id| self.closing_sub_agent(id))
                    .collect();
            }
            SessionKind::Agent => facts.sub_agents_kept = relations.sub_agents.len(),
        }
        facts.started_by = relations
            .started_by
            .iter()
            .map(|id| {
                self.agent_view_store
                    .session_context_for(id)
                    .map(|_| self.session_kind(id))
            })
            .collect();
        self.pending_close = Some(facts.clone());
        let dialog = ExitConfirmationDialog::new(facts).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    fn closing_sub_agent(&self, id: &SessionId) -> ClosingSubAgent {
        let status = if self.agent_view_store.session_context_for(id).is_none() {
            SubAgentStatus::Unknown
        } else if self.is_working(id) {
            SubAgentStatus::Running
        } else {
            SubAgentStatus::Idle
        };
        ClosingSubAgent {
            id: id.clone(),
            status,
            isolated: self.is_isolated(id),
        }
    }

    /// Fold a finished close: every closed session leaves through the
    /// session-removed cleanup; the closed focused session also releases
    /// its work unit and returns to the board; a failure is reported.
    pub(crate) fn handle_close_finished(
        &mut self,
        facts: &CloseFacts,
        closed: &[SessionId],
        failure: Option<String>,
    ) {
        for session in closed {
            if *session != facts.session {
                self.handle_session_removed(session.clone());
                continue;
            }
            let was_current = self.agent_view_store.current_session() == Some(session);
            // The work unit it was working on: the current one when it is
            // focused, else its own per-session binding.
            let work_unit = if was_current {
                self.agent_view_store
                    .current_work_unit_id()
                    .map(str::to_string)
            } else {
                self.agent_view_store
                    .work_unit_context_for(session)
                    .map(|context| context.id.clone())
            };
            self.handle_session_removed(session.clone());
            if let Some(work_unit) = &work_unit {
                self.board_store.detach_session(work_unit);
            }
            if was_current {
                self.agent_view_store.set_current_work_unit(None, None);
            }
            self.rebuild_focus_occupancy();
            let _ = self.action_tx.send(Action::BackToBoard);
        }
        if let Some(message) = failure {
            let _ = self.action_tx.send(Action::SessionOperationFailed(message));
        }
    }
}
