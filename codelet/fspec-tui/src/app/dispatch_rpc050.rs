//! App::dispatch routing for RPC-050 — work-unit binding (BoardView
//! attach path + `/detach` slash command).
//!
//! Feature files:
//!   - spec/features/work-unit-attach-binding.feature
//!   - spec/features/slash-command-detach-and-work-unit-binding.feature
//!
//! Factored out of `app/dispatch.rs` and `app/dispatch_rpc020.rs` so
//! both orchestrator files stay under the 300-LoC ceiling pinned by
//! `slash-command-detach-source-shape.feature`.
//!
//! Mirrors the spawned-task + action-bus round-trip pattern from
//! `dispatch_rpc046::/clear` and `dispatch_rpc026::/resume`.

use codelet_rpc_types::{SessionId, WorkUnitContext};

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-050: BoardView Enter path — bind the supplied work unit id to
    /// the focused AgentView session. When there is no current session
    /// the helper is a silent no-op (matches `/detach` no-session
    /// semantics). Otherwise spawns
    /// `backend.set_work_unit_context(session, Some(ctx))` and routes
    /// `Action::WorkUnitAttached(session, ctx)` on Ok or
    /// `Action::EmitSessionNotice(session, "[error] /attach failed: …")`
    /// on Err.
    pub(crate) fn handle_attach_work_unit_to_session(&mut self, work_unit_id: String) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        let Some(ctx) = self.lookup_work_unit_context(&work_unit_id) else {
            return;
        };
        // RPC-050 rule [0]: navigation to AgentView is part of the
        // attach action so the explicit dispatch path (used by the
        // future BoardView right-pane button + by Slot 4 dialog/UI
        // paths) lands on AgentView too.
        self.navigator.active_view = crate::views::ViewMode::Agent;
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let ctx_for_task = ctx;
        let session_for_task = session_id;
        let handle = tokio::spawn(async move {
            match backend
                .set_work_unit_context(session_for_task.clone(), Some(ctx_for_task.clone()))
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::WorkUnitAttached(session_for_task, ctx_for_task));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session_for_task,
                        format!("[error] /attach failed: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-050: fold a successful `set_work_unit_context(Some)` outcome
    /// into the per-session AgentViewStore map.
    pub(crate) fn handle_work_unit_attached(
        &mut self,
        session_id: SessionId,
        ctx: WorkUnitContext,
    ) {
        self.agent_view_store
            .set_work_unit_context(session_id, ctx);
    }

    /// RPC-050: fold a successful `set_work_unit_context(None)` outcome
    /// (a.k.a. `/detach`) into the AgentViewStore — clear the binding,
    /// reset the session's scrollback, and reset its TokenState.
    /// Mirrors the TS `prepareForNewSession` cleanup chain.
    pub(crate) fn handle_work_unit_detached(&mut self, session_id: SessionId) {
        self.agent_view_store
            .clear_work_unit_context(&session_id);
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
            ctx.reset_scrollback();
        }
        self.agent_view_store.reset_token_state(&session_id);
    }

    /// RPC-050: `/detach` slash command — three documented paths:
    /// (1) no active session → silent return,
    /// (2) no work unit attached → emit notice via the action bus,
    /// (3) bound → spawn the backend round-trip; Ok→WorkUnitDetached,
    ///            Err→EmitSessionNotice("[error] /detach failed: {e}").
    pub(crate) fn handle_slash_detach(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if self
            .agent_view_store
            .work_unit_context_for(&session_id)
            .is_none()
        {
            let _ = self.action_tx.send(Action::EmitSessionNotice(
                session_id,
                "[notice] /detach: no work unit attached".to_string(),
            ));
            return;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let session_for_task = session_id;
        let handle = tokio::spawn(async move {
            match backend
                .set_work_unit_context(session_for_task.clone(), None)
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::WorkUnitDetached(session_for_task));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session_for_task,
                        format!("[error] /detach failed: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Best-effort lookup of a work unit by id across the BoardStore's
    /// known columns. Returns the `WorkUnitContext` that the dispatcher
    /// would persist via `backend.set_work_unit_context(Some(_))`.
    fn lookup_work_unit_context(&self, work_unit_id: &str) -> Option<WorkUnitContext> {
        use crate::store::COLUMN_ORDER;
        for column in COLUMN_ORDER {
            if let Some(unit) = self
                .board_store
                .column_units(column)
                .iter()
                .find(|u| u.id == work_unit_id)
            {
                return Some(WorkUnitContext {
                    id: unit.id.clone(),
                    title: unit.title.clone(),
                    status: unit.status.clone(),
                });
            }
        }
        None
    }
}
