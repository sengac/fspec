//! AgentManager handler mechanism
//!
//! Feature: spec/features/agent-manager-core.feature
//!
//! Provides per-session handlers for AgentManagerTool to execute actions
//! via the session management layer in codelet-napi. Same architecture as
//! session_search/handler.rs.
//!
//! ## Architecture
//!
//! 1. Session manager registers handler via `set_agent_manager_handler(session_id, handler)`
//! 2. AgentManagerTool (constructed with session_id) calls `execute_agent_manager(session_id, action)`
//! 3. Handler accesses SessionManager directly (no TypeScript round-trip)
//! 4. Handler returns AgentManagerResult to the tool
//!
//! ## Session Association (TOOL-012 pattern)
//!
//! The tool is constructed WITH its session_id. At call time, it uses
//! `self.session_id` to look up its handler — no thread-local state.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

use super::types::{AgentManagerAction, AgentManagerResult};

/// Handler function type for agent manager execution
/// Takes an action and the calling session_id, returns the result
pub type AgentManagerHandler =
    Arc<dyn Fn(AgentManagerAction, Uuid) -> AgentManagerResult + Send + Sync>;

/// Per-session handler storage
static AGENT_MANAGER_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, AgentManagerHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the agent manager handler for a specific session
///
/// Called by session manager before agent run to configure how agent manager
/// actions are executed for this session.
pub fn set_agent_manager_handler(session_id: Uuid, handler: Option<AgentManagerHandler>) {
    if let Ok(mut guard) = AGENT_MANAGER_HANDLERS.write() {
        match handler {
            Some(h) => {
                guard.insert(session_id, h);
            }
            None => {
                guard.remove(&session_id);
            }
        }
    }
}

/// Check if an agent manager handler is configured for a specific session
pub fn has_agent_manager_handler(session_id: Uuid) -> bool {
    AGENT_MANAGER_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute an agent manager action via the handler for a specific session
///
/// Called by AgentManagerTool when the LLM invokes the tool.
pub fn execute_agent_manager(
    session_id: Uuid,
    action: AgentManagerAction,
) -> AgentManagerResult {
    let handler = match AGENT_MANAGER_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return AgentManagerResult::Error {
                error: true,
                code: "internal_error".to_string(),
                message: "Failed to acquire agent manager handlers lock".to_string(),
            };
        }
    };

    match handler {
        Some(h) => h(action, session_id),
        None => AgentManagerResult::Error {
            error: true,
            code: "internal_error".to_string(),
            message: format!(
                "Agent manager handler not configured for session {session_id} — \
                 AgentManagerTool requires session context"
            ),
        },
    }
}

/// Clear all agent manager handlers (for testing)
pub fn clear_all_agent_manager_handlers() {
    if let Ok(mut guard) = AGENT_MANAGER_HANDLERS.write() {
        guard.clear();
    }
}
