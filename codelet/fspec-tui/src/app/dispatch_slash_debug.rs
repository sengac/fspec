//! App::dispatch routing for the `/debug` slash command.
//! Introduced: RPC-055.
//!
//! Feature: spec/features/rpc055-slash-debug-dispatch.feature
//!
//! Factored into its own file so the orchestrator dispatch_slash_commands.rs's
//! `handle_slash_command` arm stays small. Mirrors the RPC-046 (`/clear`)
//! and RPC-054 (`/provider`) patterns: spawn a tokio task, await the
//! backend round-trip, route the success/error notice back through the
//! action bus into the originating session's scrollback.
//!
//! TS parity reference: `AgentView.tsx:2643` —
//! `sessionToggleDebug(currentSessionId, debugDir)` when a session is
//! active, else `toggleDebug(debugDir)` (pre-session global). The
//! pre-session global path is reachable via the new
//! `backend.set_debug_directory(path)` RPC method but no slash command
//! currently wires it (out of scope for this slice).

use crate::components::Action;

use super::state::App;

impl App {
    /// Spawn `backend.toggle_debug(session_id, debug_dir)` for the
    /// focused session and route the result back into the originating
    /// session's scrollback as a `[debug] capture toggled → {path}`
    /// notice on Ok or a `[error] /debug failed: {reason}` notice on Err.
    ///
    /// `debug_dir` resolves to the value of the `FSPEC_DEBUG_DIR`
    /// environment variable, falling back to `".fspec/debug"` when
    /// unset.
    ///
    /// With NO current session this is a silent no-op — no backend
    /// call, no notice. Mirrors the `/clear` no-session behaviour from
    /// RPC-046.
    pub(crate) fn handle_slash_debug(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let debug_dir =
            std::env::var("FSPEC_DEBUG_DIR").unwrap_or_else(|_| ".fspec/debug".to_string());
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let session_for_send = session_id;
        let handle = tokio::spawn(async move {
            let text = match backend
                .toggle_debug(session_for_send.clone(), debug_dir)
                .await
            {
                Ok(path) => format!("[debug] capture toggled \u{2192} {path}"),
                Err(e) => format!("[error] /debug failed: {e}"),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(session_for_send, text));
        });
        self.pending_tasks.push(handle);
    }

    /// Route the RPC-055 action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match so the
    /// orchestrator stays under the 300-LoC ceiling. Currently the
    /// `/debug` slash command has no dedicated Action variants (it
    /// re-uses `SlashCommandSelected(Debug)` + `EmitSessionNotice` from
    /// the established patterns), but the dispatch hook is preserved
    /// for symmetry with RPC-054's `try_dispatch_provider_settings`.
    #[allow(dead_code)]
    pub(crate) fn try_dispatch_slash_debug(&mut self, _action: &Action) -> bool {
        false
    }
}
