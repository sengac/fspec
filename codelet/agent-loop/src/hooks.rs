//! NAPI-free `SessionManagerHooks` impl for the `fspec` binary (RPC-072).
//!
//! [`FspecAgentHooks`] is installed by `codelet-fspec::common::build_service`
//! in place of the previous no-op `FspecSessionManagerHooks`. Its
//! `spawn_agent_loop` tokio-spawns [`crate::agent_loop::agent_loop`] which
//! drains the per-session `input_rx`, dispatches to the session's selected
//! `LlmProvider`, and emits `StreamChunk::Text` + `StreamChunk::Done` back
//! through `BackgroundSession::handle_output`.

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
        // RPC-072 FIX: Create the persistence manifest for this session
        // BEFORE the agent_loop starts. Without this, every persist call
        // (persist_user_message / persist_assistant_message_internal /
        // persist_tool_result_internal / persist_token_state) inside the
        // agent loop fails with "Session not found" because
        // `load_session(session.id)` only consults the on-disk session
        // store and there is no manifest file to load.
        //
        // In the napi build, the TypeScript shell calls
        // `persistence_create_session_with_provider(name, project, provider)`
        // and then `sessionCreateSessionWithId(manifest.id, model, ...)`
        // so the manifest UUID matches the BackgroundSession's UUID. The
        // fspec daemon has no TypeScript shell, so the equivalent has to
        // happen here, at the only point we know both the session.id
        // and the provider name.
        {
            let provider = session
                .provider_id
                .read()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_default();
            let project = std::path::PathBuf::from(&session.project);
            let name = session
                .name
                .read()
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default();
            let mut manifest = codelet_core::persistence::SessionManifest::with_provider(
                &name, project, &provider,
            );
            // Override the auto-generated UUID with the BackgroundSession UUID
            // so persist_user_message / load_session can find it.
            manifest.id = session.id;
            if let Err(e) = codelet_core::persistence::save_session(&manifest) {
                tracing::warn!(
                    "[RPC-072] FspecAgentHooks: failed to create persistence manifest for session {}: {} (agent loop will still run but message history will not persist)",
                    session.id,
                    e,
                );
            } else {
                tracing::debug!(
                    "[RPC-072] FspecAgentHooks: created persistence manifest for session {} (provider={})",
                    session.id,
                    provider,
                );
            }
        }

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
