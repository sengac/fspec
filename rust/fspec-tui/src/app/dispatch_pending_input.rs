//! App::dispatch routing for RPC-052 — pending-input draft persistence.
//!
//! Feature: spec/features/pending-input-draft-persistence.feature
//!
//! Hosts three impl App helpers invoked from `app/dispatch.rs`:
//!
//!   - `handle_pending_input_changed(text)`: mirrors the live buffer
//!     into `SessionContext.input_draft` synchronously AND debounces
//!     a `backend.set_pending_input(session, Some(text))` call by
//!     300ms (a second edit within the window aborts the previous
//!     in-flight task — last-write-wins). Silent no-op when no
//!     current session is open.
//!
//!   - `handle_seed_pending_input(session_id, text)`: always folds
//!     the text into the matching `SessionContext.input_draft` so the
//!     RPC-024 cycle restores the same draft; ONLY mutates the live
//!     `MultiLineInput` when the activated session is still the
//!     focused session at the moment the hydration completes.
//!
//!   - `spawn_hydrate_pending_input(session_id)`: fire-and-forget
//!     tokio task that awaits `backend.get_pending_input(session_id)`
//!     and on `Ok(Some(text))` dispatches `Action::SeedPendingInput`
//!     onto the App's action bus. Errors are swallowed via `tracing`.
//!
//! Wiring entry points:
//!   - `Action::PendingInputChanged(text)` → `handle_pending_input_changed`
//!     (routed from `app/dispatch.rs`).
//!   - `Action::SeedPendingInput { session_id, text }` → `handle_seed_pending_input`
//!     (routed from `app/dispatch.rs`).
//!   - `Action::SessionCreated(id)` arm in `app/dispatch.rs` and
//!     `handle_attach_to_session` in `app/dispatch_resume_search_views.rs` BOTH
//!     call `spawn_hydrate_pending_input(id)` after the session is
//!     registered in the AgentViewStore.
//!   - `handle_input_submitted` in `app/dispatch_slash_commands.rs` spawns a
//!     `backend.set_pending_input(session, None)` clear after the
//!     send_input + persistence path so the durable draft is purged
//!     on submit.

use std::sync::Arc;
use std::time::Duration;

use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

impl App {
    /// RPC-052: route an `Action::PendingInputChanged(text)` through the
    /// debounce + per-session draft mirror.
    pub(crate) fn handle_pending_input_changed(&mut self, text: String) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };

        // Synchronous mirror into SessionContext.input_draft so the
        // RPC-024 Shift+Left/Right cycle has the fresh draft to swap.
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
            ctx.input_draft.clone_from(&text);
        }

        // Honour the synchronous unit-test path so tests that don't
        // drive a Tokio runtime can still observe the mirror update.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }

        // Last-write-wins: cancel any pending debounced save.
        if let Some(handle) = self.pending_input_save_handle.take() {
            handle.abort();
        }

        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let id = session_id;
        let buf = text;
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if let Err(err) = backend.set_pending_input(id.clone(), Some(buf)).await {
                tracing::debug!(
                    target: "rpc052",
                    session = &id.value,
                    error = %err,
                    "set_pending_input failed (silently dropped)",
                );
            }
        });
        self.pending_input_save_handle = Some(handle);
    }

    /// RPC-052: route an `Action::SeedPendingInput { session_id, text }`
    /// through the focused-session check + SessionContext fold.
    pub(crate) fn handle_seed_pending_input(&mut self, session_id: SessionId, text: String) {
        // ALWAYS fold the text into the matching SessionContext.input_draft.
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
            ctx.input_draft.clone_from(&text);
        }
        // ONLY seed the live MultiLineInput when the activated session is
        // still the focused session at the moment the result arrives.
        let focused = self.agent_view_store.current_session().cloned();
        if focused.as_ref() == Some(&session_id) {
            self.navigator.agent.input.set_value(&text);
            self.should_render = true;
        }
    }

    /// RPC-052: clear the durable per-session pending-input draft
    /// after an `Action::InputSubmitted` lands. Fire-and-forget —
    /// errors are silently logged via tracing. Called from
    /// `handle_input_submitted` in `app/dispatch_slash_commands.rs` so the
    /// caller stays under the 300-LoC ceiling.
    pub(crate) fn spawn_clear_pending_input(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.set_pending_input(session_id.clone(), None).await {
                tracing::debug!(
                    target: "rpc052",
                    session = &session_id.value,
                    error = %err,
                    "set_pending_input(None) on submit failed (silently dropped)",
                );
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-052: spawn a fire-and-forget task that hydrates the
    /// per-session draft via `backend.get_pending_input(session_id)`
    /// and dispatches `Action::SeedPendingInput` on Ok(Some).
    pub(crate) fn spawn_hydrate_pending_input(&mut self, session_id: SessionId) {
        // Honour the synchronous unit-test path so tests that don't
        // drive a Tokio runtime can still observe state without
        // panicking.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            match backend.get_pending_input(id.clone()).await {
                Ok(Some(text)) => {
                    let _ = action_tx.send(Action::SeedPendingInput {
                        session_id: id,
                        text,
                    });
                }
                Ok(None) => {
                    // No durable draft — nothing to seed.
                }
                Err(err) => {
                    tracing::debug!(
                        target: "rpc052",
                        session = &id.value,
                        error = %err,
                        "get_pending_input failed (silently dropped)",
                    );
                }
            }
        });
        self.pending_tasks.push(handle);
    }
}
