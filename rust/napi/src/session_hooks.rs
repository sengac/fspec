//! Session-manager lifecycle hooks adapter (RPC-043).
//!
//! Extracted from `rust/napi/src/session_manager.rs` by RPC-043.
//! `NapiSessionManagerHooks` is the napi-side implementation of
//! [`codelet_sessions::session_manager::SessionManagerHooks`]. It wires
//! session lifecycle events to the napi-side helpers that own the agent
//! loop (`crate::agent_loop`), the scheduler (`crate::scheduler`), and
//! the footer poller (`crate::footer_poller`).
//!
//! `install_napi_session_manager_hooks` is called once from
//! `crate::session_bindings::session_set_global_chunk_callback` so the
//! TS-driven session creation path continues to spawn the agent loop,
//! the scheduler, the footer poller, and the IsolationStateChange
//! fan-out exactly as it did before RPC-040.

use codelet_sessions::session_manager::SessionManager;

#[derive(Default)]
pub struct NapiSessionManagerHooks;

impl codelet_sessions::session_manager::SessionManagerHooks for NapiSessionManagerHooks {
    fn spawn_agent_loop(
        &self,
        session: std::sync::Arc<codelet_sessions::background_session::BackgroundSession>,
        input_rx: tokio::sync::mpsc::Receiver<codelet_sessions::background_session::PromptInput>,
        mcp_injection_rx: tokio::sync::mpsc::Receiver<codelet_tools::McpInjection>,
    ) {
        tokio::spawn(async move {
            crate::agent_loop::agent_loop(session, input_rx, mcp_injection_rx).await;
        });
    }

    fn spawn_scheduler(&self, project: String, rt: tokio::runtime::Handle) {
        let _h = crate::scheduler::spawn_scheduler(project, &rt);
        // The returned JoinHandle is dropped intentionally — the napi
        // SessionManager already tracks a sentinel handle for the
        // "scheduler started" condition.
    }

    fn ensure_scheduler_running_for_loop(&self, project: String, rt: tokio::runtime::Handle) {
        let _h = crate::scheduler::spawn_scheduler(project, &rt);
    }

    fn spawn_footer_poller(&self, session_id: String, cwd: String, worktree_path: Option<String>) {
        crate::footer_poller::spawn_footer_poller(session_id, cwd, worktree_path);
    }

    fn stop_footer_poller(&self, session_id: &str) {
        crate::footer_poller::stop_footer_poller(session_id);
    }

    fn cleanup_session_loops(&self, session_id: uuid::Uuid) {
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(async move {
                crate::scheduler::LoopStore::instance()
                    .remove_for_session(session_id)
                    .await;
            });
        }
    }
}

/// Install [`NapiSessionManagerHooks`] on the global SessionManager
/// singleton. Called once from
/// `crate::session_bindings::session_set_global_chunk_callback` so the
/// existing TS-driven session creation path continues to spawn the
/// agent loop, the scheduler, the footer poller, and the
/// IsolationStateChange fan-out exactly as it did before RPC-040.
pub(crate) fn install_napi_session_manager_hooks() {
    SessionManager::instance().set_hooks(std::sync::Arc::new(NapiSessionManagerHooks));
}
