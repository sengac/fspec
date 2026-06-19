//! App::dispatch routing for the AgentView exit-confirmation choices
//! (RPC-098).
//!
//! Feature: spec/features/agentview-esc-exit-confirmation-dialog.feature
//!
//! Routing decision tree for `Action::AgentExitChoice { choice }`:
//!
//! ```text
//! choice = Cancel        → no-op (dialog removed via Callback)
//! choice = Detach        → dispatch Action::BackToBoard; session keeps
//!                          running in the backend (TS parity with
//!                          GlobalSessionStreamManager detach semantics)
//! choice = CloseSession  → mirrors TS `destroySession()` orchestrator
//!                          at src/tui/services/sessionService.ts:620-647
//!                          step-for-step:
//!
//!                          1. SNAPSHOT — capture work-unit binding +
//!                             session id BEFORE any mutation so we have
//!                             the keys we need to drop downstream.
//!
//!                          2. BoardStore detach (TS step 2 — line 637:
//!                             `fspecStore.detachSession(workUnitId)`).
//!                             Without it BoardView::selected_session
//!                             (views/board.rs:182-185) keeps returning
//!                             the destroyed SessionId so Shift+Right on
//!                             the same work unit routes to a dead
//!                             session.
//!
//!                          3. AgentViewStore::open_sessions removal
//!                             (canonical Rust equivalent of the local
//!                             open-session list cleanup — mirrors the
//!                             pattern at handle_confirm_delete_session
//!                             dispatch_rpc026.rs:249). Without it the
//!                             destroyed SessionContext stays in
//!                             open_sessions and Shift+Left/Right cycle
//!                             via navigate_prev/navigate_next /
//!                             first_open_session_id keeps surfacing the
//!                             dead session.
//!
//!                          4. Current-work-unit pointer clear (TS step
//!                             3 — line 642:
//!                             `sessionStore.setCurrentWorkUnit(null,
//!                             null)`). Without it the AgentViewStore
//!                             still believes the destroyed session's
//!                             work unit is "current", which can drive
//!                             stale chrome (SessionHeader `#N (WU-ID:
//!                             status)` segment) and a spurious
//!                             AttachWorkUnitToSession round-trip the
//!                             next time a fresh session is created.
//!
//!                          5. Backend destroy (TS step 1 — line 627:
//!                             `sessionManagerDestroy(sessionId)`).
//!                             Spawned via tokio::spawn so the
//!                             BackToBoard transition stays
//!                             synchronous; the IndexMap removal lands
//!                             on the next tick of the runtime. Rust's
//!                             cycle source is `open_sessions` (already
//!                             cleared synchronously above), so the
//!                             async timing here is safe.
//!
//!                          6. Dispatch Action::BackToBoard.
//! ```

use crate::components::exit_confirmation_dialog::ExitChoice;
use crate::components::Action;

use super::state::App;

impl App {
    /// Route `Action::AgentExitChoice { choice }` per the table above.
    pub(crate) fn handle_agent_exit_choice(&mut self, choice: ExitChoice) {
        match choice {
            ExitChoice::Cancel => {
                // No-op. The dialog's Callback already removed it from the
                // compositor; the AgentView remains active.
            }
            ExitChoice::Detach => {
                let _ = self.action_tx.send(Action::BackToBoard);
            }
            ExitChoice::CloseSession => {
                if let Some(session) = self.agent_view_store.current_session().cloned() {
                    // Step 1: snapshot the work-unit binding BEFORE
                    // mutating any store so we still know which key to
                    // drop in BoardStore.
                    let work_unit_id = self
                        .agent_view_store
                        .current_work_unit_id()
                        .map(str::to_string);

                    // Step 2: BoardStore detach (TS parity step 2 —
                    // src/tui/services/sessionService.ts:637). Mirrors
                    // `fspecStore.detachSession(workUnitId)`. Without
                    // this, BoardView::selected_session
                    // (views/board.rs:182-185) keeps returning this
                    // SessionId and Shift+Right would navigate back to
                    // the destroyed session.
                    if let Some(wu_id) = &work_unit_id {
                        self.board_store.detach_session(wu_id);
                    }

                    // Step 3: AgentViewStore::open_sessions removal
                    // (canonical Rust local cleanup — mirrors the
                    // pattern at handle_confirm_delete_session
                    // dispatch_rpc026.rs:249). The Rust cycle source is
                    // `open_sessions` (NOT the backend SessionManager
                    // IndexMap that TS reads via
                    // `sessionGetNext/Prev`), so this synchronous
                    // removal is what guarantees Shift+Left/Right and
                    // first_open_session_id drop the dead session
                    // BEFORE the user can navigate again.
                    self.agent_view_store.remove_session_if_open(&session);

                    // Step 4: current-work-unit pointer clear (TS
                    // parity step 3 — sessionService.ts:642). Mirrors
                    // `sessionStore.setCurrentWorkUnit(null, null)`.
                    // Without this the AgentViewStore still believes
                    // the destroyed session's work unit is current,
                    // which paints stale `#N (WU-ID: status)` chrome on
                    // the SessionHeader and can drive a spurious
                    // AttachWorkUnitToSession round-trip the next time
                    // a fresh session is created.
                    self.agent_view_store.set_current_work_unit(None, None);

                    // Step 5: backend destroy (TS parity step 1 —
                    // sessionService.ts:627 —
                    // `sessionManagerDestroy(sessionId)`). Spawned so
                    // BackToBoard stays synchronous; the cycle source
                    // (`open_sessions`) was already cleared in step 3.
                    let backend = self.backend.clone();
                    let session_for_destroy = session.clone();
                    let handle = tokio::spawn(async move {
                        let _ = backend.destroy_session(session_for_destroy).await;
                    });
                    self.pending_tasks.push(handle);
                }
                // Step 6.
                let _ = self.action_tx.send(Action::BackToBoard);
            }
        }
    }
}
