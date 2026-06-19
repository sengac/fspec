//! App::dispatch routing for RPC-046 — `/clear` slash command end-to-end.
//!
//! Feature: spec/features/slash-command-clear.feature
//! Feature: spec/features/rpc074-clear-ts-parity.feature
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling pinned by `rpc024-source-shape.feature`.
//!
//! TS parity (RPC-074): mirrors
//! `src/tui/components/AgentView.tsx:1554-1564` (handleClearCommand) —
//! blanks the input + scrollback locally, then calls
//! `backend.clear_history(session_id)`. NO scrollback notice line is
//! pushed by the dispatcher: the reactive UI reset is driven by the
//! `StreamChunk::SessionStateChange { state: Cleared }` chunk emitted
//! by `BackgroundSession::clear_history` (TS TUI-066 contract). Errors
//! go to `tracing::error!`, never to user-visible scrollback.
//!
//! `handle_emit_session_notice` stays in this file because it is still
//! used by other slash commands (e.g. `/compact`) to route notices back
//! to the originating SessionContext via `Action::EmitSessionNotice`.

use codelet_rpc_types::SessionId;

use super::state::App;

impl App {
    /// Push `text` into `session_id`'s scrollback (per-session, not
    /// focused-session). Used to fold an `Action::EmitSessionNotice`
    /// back into the originating SessionContext.
    pub(crate) fn handle_emit_session_notice(&mut self, session_id: &SessionId, text: String) {
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(session_id) {
            ctx.push_line(text);
        }
    }

    /// RPC-046 / RPC-074: `/clear` slash command body. Resets the
    /// focused session's local scrollback + input synchronously
    /// (optimistic UI), then (when a current session exists) spawns a
    /// tokio task that awaits `backend.clear_history(session_id)`. On
    /// Err the failure is logged via `tracing::error!` ONLY — no
    /// `[error] /clear failed` line is pushed to scrollback (TS parity
    /// with logger.error in `handleClearCommand`). Bare `/clear` with
    /// no current session is a silent no-op.
    pub(crate) fn handle_slash_clear(&mut self) {
        self.navigator
            .agent
            .reset_scrollback(&mut self.agent_view_store);
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = backend.clear_history(session_id.clone()).await {
                // TS parity (RPC-074): error goes to tracing, NOT to
                // scrollback. The source-shape regression test asserts
                // this file is free of divergent literals.
                tracing::error!(?session_id, error = %e, "/clear: backend.clear_history failed");
            }
        });
        self.pending_tasks.push(handle);
    }
}
