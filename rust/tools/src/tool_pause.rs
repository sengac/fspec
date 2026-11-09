//! Tool pause mechanism (BUG-127: per-session isolation)
//!
//! Provides a generic pause/resume API for tools that need user interaction.
//! Supports two pause kinds:
//! - Continue: Press Enter to resume
//! - Confirm: Press Y to approve, N to deny
//!
//! Pause state is PER-SESSION. Handlers are keyed by `session_id: Uuid` via
//! [`SessionRegistry`] to prevent concurrent sessions from overwriting each
//! other's handlers.

use std::sync::Arc;

use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::session_registry::SessionRegistry;

/// Kind of pause requested by a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseKind {
    Continue,
    Confirm,
    Triple, // BLOCK-007: For blocklist prompts (Allow Once / Allow Session / Deny)
}

/// Request to pause tool execution
#[derive(Debug, Clone)]
pub struct PauseRequest {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
}

/// Response from user after pause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseResponse {
    Resumed,
    Approved,
    Denied,
    Interrupted,
    AllowOnce,    // BLOCK-007: For triple mode - permit once, prompt again next time
    AllowSession, // BLOCK-007: For triple mode - permit for entire session
}

/// Current pause state (for UI display)
#[derive(Debug, Clone)]
pub struct PauseState {
    pub kind: PauseKind,
    pub tool_name: String,
    pub message: String,
    pub details: Option<String>,
    /// BUG-291: the tool call that raised the pause, when one is running.
    pub tool_call_id: Option<String>,
}

impl From<PauseRequest> for PauseState {
    fn from(req: PauseRequest) -> Self {
        Self {
            kind: req.kind,
            tool_name: req.tool_name,
            message: req.message,
            details: req.details,
            tool_call_id: None,
        }
    }
}

impl PauseState {
    /// BUG-291: the state for a pause `session_id` raised, naming the
    /// session's active tool call (see [`set_active_tool_call`]).
    pub fn for_session(session_id: Uuid, request: PauseRequest) -> Self {
        Self {
            tool_call_id: active_tool_call(session_id),
            ..request.into()
        }
    }
}

/// BUG-291: the tool call each session is currently executing, recorded by
/// the stream loop when a ToolCall streams and cleared when its result lands.
static ACTIVE_TOOL_CALLS: Lazy<SessionRegistry<String>> = Lazy::new(SessionRegistry::new);

/// Record (or clear) the tool call `session_id` is executing.
pub fn set_active_tool_call(session_id: Uuid, tool_call_id: Option<String>) {
    ACTIVE_TOOL_CALLS.set(session_id, tool_call_id);
}

/// The tool call `session_id` is executing, if any.
pub fn active_tool_call(session_id: Uuid) -> Option<String> {
    ACTIVE_TOOL_CALLS.get(&session_id)
}

pub type PauseHandler = Arc<dyn Fn(PauseRequest) -> PauseResponse + Send + Sync>;

/// Per-session handler storage (BUG-127: replaced global singleton).
static PAUSE_HANDLERS: Lazy<SessionRegistry<PauseHandler>> = Lazy::new(SessionRegistry::new);

/// Register or clear a pause handler for a specific session.
pub fn set_pause_handler(session_id: Uuid, handler: Option<PauseHandler>) {
    PAUSE_HANDLERS.set(session_id, handler);
}

/// Pause tool execution and wait for user response (per-session).
///
/// If a handler is registered for the given session_id, calls it.
/// If no handler is registered, returns `PauseResponse::Resumed`.
pub fn pause_for_user(session_id: Uuid, request: PauseRequest) -> PauseResponse {
    match PAUSE_HANDLERS.get(&session_id) {
        Some(h) => h(request),
        None => PauseResponse::Resumed,
    }
}

/// Check if a pause handler is registered for a specific session.
pub fn has_pause_handler(session_id: Uuid) -> bool {
    PAUSE_HANDLERS.has(&session_id)
}

#[cfg(test)]
mod tests;
