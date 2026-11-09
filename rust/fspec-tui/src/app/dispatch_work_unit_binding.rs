//! App::dispatch routing for RPC-050 — work-unit binding (BoardView
//! attach path).
//!
//! Feature files:
//!   - spec/features/work-unit-attach-binding.feature
//!   - spec/features/session-work-unit-focus-projection.feature
//!
//! Factored out of `app/dispatch.rs` and `app/dispatch_slash_commands.rs` so
//! both orchestrator files stay under the 300-LoC ceiling pinned by
//! `slash-command-detach-source-shape.feature`.
//!
//! BUG-205: the `/detach` slash command (and its slash-command handler +
//! detached-state fold helpers) was removed — the command's only effect was
//! clearing the session's work-unit binding, and with the exit dialog's
//! Detach option gone it had no UI home. Only the attach path remains.
//!
//! Mirrors the spawned-task + action-bus round-trip pattern from
//! `dispatch_slash_clear::/clear` and `dispatch_resume_search_views::/resume`.

use codelet_rpc_types::{SessionId, WorkUnitContext};

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-050 / BUG-230: bind `work_unit_id` to `session_id` through
    /// `backend.set_work_unit_context`, then read the session's work-unit
    /// focus back (`Action::WorkUnitFocusChanged`).
    pub(crate) fn attach_work_unit_to_session(
        &mut self,
        session_id: SessionId,
        work_unit_id: String,
    ) {
        let Some(ctx) = self.lookup_work_unit_context(&work_unit_id) else {
            return;
        };
        // RPC-050 rule [0]: navigation to AgentView is part of the
        // attach action so the explicit dispatch path (used by the
        // future BoardView right-pane button + by Slot 4 dialog/UI
        // paths) lands on AgentView too.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let ctx_for_task = ctx;
        let session_for_task = session_id;
        let handle = tokio::spawn(async move {
            match backend
                .set_work_unit_context(session_for_task.clone(), Some(ctx_for_task))
                .await
            {
                Ok(()) => match backend.get_work_unit_focus(session_for_task.clone()).await {
                    Ok(focus) => {
                        let _ =
                            action_tx.send(Action::WorkUnitFocusChanged(session_for_task, focus));
                    }
                    Err(error) => {
                        let _ = action_tx.send(Action::EmitSessionNotice(
                            session_for_task,
                            format!("[error] /attach failed: {error}"),
                        ));
                    }
                },
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

    /// Best-effort lookup of a work unit by id across the BoardStore's
    /// known columns. Returns the `WorkUnitContext` that the dispatcher
    /// would persist via `backend.set_work_unit_context(Some(_))`.
    pub(crate) fn lookup_work_unit_context(&self, work_unit_id: &str) -> Option<WorkUnitContext> {
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
