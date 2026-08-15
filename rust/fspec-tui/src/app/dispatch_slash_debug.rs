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
//! active, else `toggleDebug(debugDir)` (pre-session global).
//!
//! RPC-430: resolves debug directory to `~/.fspec` (matching TS
//! `getFspecUserDir()`) and supports pre-session toggle when no session
//! is active (mirrors TS `AgentView.tsx:2713-2715`).

use crate::components::Action;

use super::state::App;

/// Resolve the debug-capture directory.
///
/// RPC-430: matches TypeScript's `getFspecUserDir()` — prefers the
/// `FSPEC_DEBUG_DIR` environment variable override, falls back to
/// `~/.fspec` derived from `HOME`, then a bare `.fspec` fallback.
fn resolve_debug_dir() -> String {
    if let Ok(custom) = std::env::var("FSPEC_DEBUG_DIR") {
        return custom;
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{home}/.fspec");
    }
    ".fspec".to_string()
}

impl App {
    /// Spawn `backend.toggle_debug(session_id, debug_dir)` for the
    /// focused session and route the result back into the originating
    /// session's scrollback as a `[debug] capture toggled → {path}`
    /// notice on Ok or a `[error] /debug failed: {reason}` notice on Err.
    ///
    /// `debug_dir` resolves via [`resolve_debug_dir`] — `FSPEC_DEBUG_DIR`
    /// override, then `~/.fspec`, then `.fspec` fallback.
    ///
    /// RPC-430: when there is NO current session the handler toggles
    /// `pre_session_debug_enabled` and emits a scrollback notice (mirrors
    /// TypeScript's pre-session `toggleDebug(debugDir)` path).
    pub(crate) fn handle_slash_debug(&mut self) {
        let debug_dir = resolve_debug_dir();

        // RPC-430: pre-session toggle path — mirrors TS AgentView.tsx:2713-2715
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            // No active session: toggle the pre-session flag and emit notice.
            self.pre_session_debug_enabled = !self.pre_session_debug_enabled;
            let text = format!("[debug] capture toggled \u{2192} {debug_dir}");
            // Emit notice to a placeholder session so the user sees feedback.
            // The notice is emitted via Custom action since there's no session.
            let _ = self
                .action_tx
                .send(Action::Custom(format!("[notice] {text}")));
            return;
        };

        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
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
