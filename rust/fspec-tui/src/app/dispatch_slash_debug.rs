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

use crate::components::notification_dialog::{NotificationDialog, NOTIFICATION_DIALOG_ID};
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
    /// `pre_session_debug_enabled` (mirrors TypeScript's pre-session
    /// `toggleDebug(debugDir)` path); TUI-127: a notification says whether
    /// capture is on for the next session.
    pub(crate) fn handle_slash_debug(&mut self) {
        let debug_dir = resolve_debug_dir();

        // RPC-430: pre-session toggle path — mirrors TS AgentView.tsx:2713-2715
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            // TUI-127: no session has a scrollback to take the notice, so a
            // notification says what changed. The next session is started
            // with capture on (dispatch_create_session_dialog.rs); the daemon
            // picks its directory, so none is named.
            self.pre_session_debug_enabled = !self.pre_session_debug_enabled;
            let text = if self.pre_session_debug_enabled {
                "Debug capture will be on for the next session"
            } else {
                "Debug capture is off"
            };
            // A second /debug replaces the first notification.
            let _ = self.compositor.remove(NOTIFICATION_DIALOG_ID);
            self.compositor.push(Box::new(
                NotificationDialog::info(text).with_action_tx(self.action_tx.clone()),
            ));
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
}
