//! Pre-tool-use lifecycle hook check
//!
//! Global per-session callback mechanism for running pre_tool_use hooks
//! before a tool executes. Follows the same pattern as tool_pause.rs
//! and fspec_handler.rs — a global handler registered from the NAPI layer.
//!
//! The handler is called synchronously from within each tool's `call()` method.
//! The NAPI layer implements the handler by running the async lifecycle hook
//! engine via `tokio::task::block_in_place` + `Handle::block_on`.

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::Value;
use uuid::Uuid;

/// Decision returned by a pre_tool_use hook check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreToolHookDecision {
    /// Hook says: allow the tool call (skip permission prompts).
    Allow,
    /// Hook says: deny the tool call with a reason.
    Deny(String),
    /// Hook says: no opinion — proceed with normal permission checks.
    Continue,
}

/// Callback type for pre_tool_use hook execution.
///
/// Takes (session_id, tool_name, tool_input_json) and returns a decision.
/// The implementation in session_manager.rs calls the lifecycle hook engine.
pub type PreToolHookHandler =
    std::sync::Arc<dyn Fn(Uuid, &str, &Value) -> PreToolHookDecision + Send + Sync>;

/// Per-session hook handler store.
static SESSION_HANDLERS: RwLock<Option<HashMap<Uuid, PreToolHookHandler>>> = RwLock::new(None);

/// Register a pre_tool_use hook handler for a session.
///
/// Called from session_manager.rs when creating a session that has lifecycle hooks.
pub fn register_pre_tool_hook(session_id: Uuid, handler: PreToolHookHandler) {
    if let Ok(mut guard) = SESSION_HANDLERS.write() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(session_id, handler);
    }
}

/// Unregister a pre_tool_use hook handler for a session.
///
/// Called from session_manager.rs when destroying a session.
pub fn unregister_pre_tool_hook(session_id: Uuid) {
    if let Ok(mut guard) = SESSION_HANDLERS.write() {
        if let Some(map) = guard.as_mut() {
            map.remove(&session_id);
        }
    }
}

/// Check pre_tool_use hooks for a tool call.
///
/// Returns Ok(()) if the tool should proceed (Allow or Continue).
/// Returns Err(reason) if the tool should be blocked (Deny).
///
/// This is called at the top of each tool's `call()` method.
/// If no handler is registered for the session, returns Ok(()) (no-op).
pub fn pre_tool_hook_check(
    session_id: Uuid,
    tool_name: &str,
    tool_input: &Value,
) -> Result<PreToolHookDecision, String> {
    let handler = {
        let guard = match SESSION_HANDLERS.read() {
            Ok(g) => g,
            Err(_) => return Ok(PreToolHookDecision::Continue),
        };
        match guard.as_ref() {
            Some(map) => map.get(&session_id).cloned(),
            None => None,
        }
    };

    let Some(handler) = handler else {
        return Ok(PreToolHookDecision::Continue);
    };

    let decision = handler(session_id, tool_name, tool_input);
    match decision {
        PreToolHookDecision::Deny(reason) => Err(reason),
        other => Ok(other),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Arc;

    fn cleanup() {
        if let Ok(mut guard) = SESSION_HANDLERS.write() {
            *guard = None;
        }
    }

    #[test]
    #[serial]
    fn test_no_handler_returns_continue() {
        cleanup();
        let session_id = Uuid::new_v4();
        let result = pre_tool_hook_check(session_id, "Bash", &Value::Null);
        assert_eq!(result, Ok(PreToolHookDecision::Continue));
        cleanup();
    }

    #[test]
    #[serial]
    fn test_handler_allow_returns_ok() {
        cleanup();
        let session_id = Uuid::new_v4();
        let handler: PreToolHookHandler =
            Arc::new(|_sid, _name, _input| PreToolHookDecision::Allow);
        register_pre_tool_hook(session_id, handler);

        let result = pre_tool_hook_check(session_id, "Bash", &Value::Null);
        assert_eq!(result, Ok(PreToolHookDecision::Allow));
        cleanup();
    }

    #[test]
    #[serial]
    fn test_handler_deny_returns_err() {
        cleanup();
        let session_id = Uuid::new_v4();
        let handler: PreToolHookHandler = Arc::new(|_sid, _name, _input| {
            PreToolHookDecision::Deny("blocked by policy".to_string())
        });
        register_pre_tool_hook(session_id, handler);

        let result = pre_tool_hook_check(session_id, "Bash", &Value::Null);
        assert_eq!(result, Err("blocked by policy".to_string()));
        cleanup();
    }

    #[test]
    #[serial]
    fn test_handler_receives_correct_args() {
        cleanup();
        let session_id = Uuid::new_v4();
        let captured_session = Arc::new(std::sync::Mutex::new(Uuid::nil()));
        let captured_name = Arc::new(std::sync::Mutex::new(String::new()));
        let cs = captured_session.clone();
        let cn = captured_name.clone();

        let handler: PreToolHookHandler = Arc::new(move |sid, name, _input| {
            *cs.lock().unwrap() = sid;
            *cn.lock().unwrap() = name.to_string();
            PreToolHookDecision::Continue
        });
        register_pre_tool_hook(session_id, handler);

        let _ = pre_tool_hook_check(session_id, "WebSearch", &Value::Null);

        assert_eq!(*captured_session.lock().unwrap(), session_id);
        assert_eq!(*captured_name.lock().unwrap(), "WebSearch");
        cleanup();
    }

    #[test]
    #[serial]
    fn test_unregister_removes_handler() {
        cleanup();
        let session_id = Uuid::new_v4();
        let handler: PreToolHookHandler =
            Arc::new(|_sid, _name, _input| PreToolHookDecision::Deny("no".to_string()));
        register_pre_tool_hook(session_id, handler);
        unregister_pre_tool_hook(session_id);

        let result = pre_tool_hook_check(session_id, "Bash", &Value::Null);
        assert_eq!(result, Ok(PreToolHookDecision::Continue));
        cleanup();
    }

    #[test]
    #[serial]
    fn test_different_sessions_have_different_handlers() {
        cleanup();
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let handler_a: PreToolHookHandler =
            Arc::new(|_sid, _name, _input| PreToolHookDecision::Allow);
        let handler_b: PreToolHookHandler =
            Arc::new(|_sid, _name, _input| PreToolHookDecision::Deny("nope".to_string()));

        register_pre_tool_hook(session_a, handler_a);
        register_pre_tool_hook(session_b, handler_b);

        let result_a = pre_tool_hook_check(session_a, "Bash", &Value::Null);
        let result_b = pre_tool_hook_check(session_b, "Bash", &Value::Null);

        assert_eq!(result_a, Ok(PreToolHookDecision::Allow));
        assert_eq!(result_b, Err("nope".to_string()));
        cleanup();
    }
}
