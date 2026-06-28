//! AgentManager handler mechanism
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-await-idle.feature
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
//! For async actions (await_idle), a separate async handler is registered and
//! invoked via `execute_agent_manager_async()`.
//!
//! ## Session Association (TOOL-012 pattern)
//!
//! The tool is constructed WITH its session_id. At call time, it uses
//! `self.session_id` to look up its handler — no thread-local state.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

use super::types::{AgentManagerAction, AgentManagerResult};

/// Handler function type for agent manager execution (sync actions)
/// Takes an action and the calling session_id, returns the result
pub type AgentManagerHandler =
    Arc<dyn Fn(AgentManagerAction, Uuid) -> AgentManagerResult + Send + Sync>;

/// Async handler function type for agent manager execution (AMGR-015)
/// Used for actions that need to .await (e.g., await_idle)
pub type AgentManagerAsyncHandler = Arc<
    dyn Fn(AgentManagerAction, Uuid) -> Pin<Box<dyn Future<Output = AgentManagerResult> + Send>>
        + Send
        + Sync,
>;

/// Per-session handler storage (sync)
static AGENT_MANAGER_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, AgentManagerHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Per-session async handler storage (AMGR-015)
static AGENT_MANAGER_ASYNC_HANDLERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, AgentManagerAsyncHandler>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

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

/// Set the async agent manager handler for a specific session (AMGR-015)
///
/// Called alongside `set_agent_manager_handler` to register the async handler
/// for actions like `await_idle` that require async execution.
pub fn set_agent_manager_async_handler(
    session_id: Uuid,
    handler: Option<AgentManagerAsyncHandler>,
) {
    if let Ok(mut guard) = AGENT_MANAGER_ASYNC_HANDLERS.write() {
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

/// Execute an agent manager action via the handler for a specific session (sync)
///
/// Called by AgentManagerTool for all non-async actions.
pub fn execute_agent_manager(session_id: Uuid, action: AgentManagerAction) -> AgentManagerResult {
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

/// Execute an agent manager action via the async handler for a specific session (AMGR-015)
///
/// Called by AgentManagerTool for async actions like `await_idle`.
pub async fn execute_agent_manager_async(
    session_id: Uuid,
    action: AgentManagerAction,
) -> AgentManagerResult {
    let handler = match AGENT_MANAGER_ASYNC_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return AgentManagerResult::Error {
                error: true,
                code: "internal_error".to_string(),
                message: "Failed to acquire agent manager async handlers lock".to_string(),
            };
        }
    };

    match handler {
        Some(h) => h(action, session_id).await,
        None => AgentManagerResult::Error {
            error: true,
            code: "internal_error".to_string(),
            message: format!(
                "Agent manager async handler not configured for session {session_id} — \
                 await_idle requires async session context"
            ),
        },
    }
}

/// Clear all agent manager handlers (for testing)
pub fn clear_all_agent_manager_handlers() {
    if let Ok(mut guard) = AGENT_MANAGER_HANDLERS.write() {
        guard.clear();
    }
    if let Ok(mut guard) = AGENT_MANAGER_ASYNC_HANDLERS.write() {
        guard.clear();
    }
}
