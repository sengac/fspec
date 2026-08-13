//! NAPI-free `SessionManagerHooks` impl for the `fspec` binary (RPC-072).
//!
//! [`FspecAgentHooks`] is installed by `codelet-fspec::common::build_service`
//! in place of the previous no-op `FspecSessionManagerHooks`. Its
//! `spawn_agent_loop` tokio-spawns [`crate::agent_loop::agent_loop`] which
//! drains the per-session `input_rx`, dispatches to the session's selected
//! `LlmProvider`, and emits `StreamChunk::Text` + `StreamChunk::Done` back
//! through `BackgroundSession::handle_output`.
//!
//! **Session manifest creation** is handled by
//! `SessionManager::create_session_with_id` (codelet-sessions) which saves
//! the manifest with the full provider/model string (e.g., "anthropic/claude-sonnet-4")
//! BEFORE calling `spawn_agent_loop`. The hooks implementation does NOT create
//! or overwrite the manifest — that responsibility was removed by RPC-423.

use std::sync::Arc;

use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::session_manager::SessionManagerHooks;
use codelet_tools::McpInjection;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent_loop::agent_loop;

/// `SessionManagerHooks` impl that wires session lifecycle into the
/// NAPI-free agent loop (RPC-072).
///
/// Constructed via [`FspecAgentHooks::default`] — no per-instance state
/// is needed for the minimum-viable RPC-072 scope because each session
/// already carries its provider/model selection inside `session.inner`.
#[derive(Default)]
pub struct FspecAgentHooks;

impl FspecAgentHooks {
    /// Construct a fresh hooks impl. Stateless.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SessionManagerHooks for FspecAgentHooks {
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
        // RPC-072: handle off to the NAPI-free agent loop. The future
        // is parked on whatever runtime is current at session-creation
        // time — that's the same tokio runtime SharedFspecService spawns
        // its background tasks on, so `BackgroundSession::handle_output`
        // (which fans chunks out via the manager-owned broadcast) is
        // reachable from the same scheduler.
        let session_for_log = session.id;
        tokio::spawn(async move {
            tracing::debug!(
                "[RPC-072] FspecAgentHooks::spawn_agent_loop: starting agent_loop for session {}",
                session_for_log,
            );
            agent_loop(session, input_rx, mcp_injection_rx).await;
            tracing::debug!(
                "[RPC-072] FspecAgentHooks::spawn_agent_loop: agent_loop exited for session {}",
                session_for_log,
            );
        });
    }

    fn spawn_scheduler(&self, _project: String, _rt: tokio::runtime::Handle) {
        // Scheduler engine is wired separately by RPC-058 — out of
        // RPC-072 scope. No-op here so a session can still be created
        // without a scheduler attached.
    }

    fn ensure_scheduler_running_for_loop(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn spawn_footer_poller(
        &self,
        _session_id: String,
        _cwd: String,
        _worktree_path: Option<String>,
    ) {
        // Footer poller stays NAPI-only for now (TUI-091).
    }

    fn stop_footer_poller(&self, _session_id: &str) {}

    /// RPC-059 parity: abort + remove every registered loop bound to the
    /// destroyed session so the process-global `LoopStore` does not
    /// keep orphaned tokio tasks alive. This is the only behaviour that
    /// `FspecSessionManagerHooks` (the impl we replace) actually did,
    /// so we preserve it here byte-for-byte.
    fn cleanup_session_loops(&self, session_id: Uuid) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                codelet_core::loops::LoopStore::instance()
                    .remove_for_session(session_id)
                    .await;
            });
        }
    }
}
