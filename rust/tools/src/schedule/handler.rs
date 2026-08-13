//! Schedule handler registry — per-session handler storage
//!
//! Feature: spec/features/schedule-ai-tool.feature
//!
//! Follows the handler-delegated pattern used by SessionSearch, AgentManager,
//! and InjectSummary. The handler is registered per-session during agent_loop
//! setup and cleaned up on session teardown.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use uuid::Uuid;

use super::types::{ScheduleRequest, ScheduleResult};

/// Handler function type for schedule command execution.
///
/// Takes a ScheduleRequest and returns a ScheduleResult.
/// The handler is synchronous — async work uses block_in_place internally.
pub type ScheduleHandler = Arc<dyn Fn(ScheduleRequest) -> ScheduleResult + Send + Sync>;

/// Per-session handler storage.
/// Uses a global HashMap keyed by session UUID — handlers are shared across threads.
static SCHEDULE_HANDLERS: Lazy<RwLock<HashMap<Uuid, ScheduleHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the schedule handler for a specific session.
///
/// Called by session manager before agent run to configure how schedule
/// commands are executed for this session.
///
/// Pass `None` to remove the handler (cleanup on session teardown).
pub fn set_schedule_handler(session_id: Uuid, handler: Option<ScheduleHandler>) {
    if let Ok(mut guard) = SCHEDULE_HANDLERS.write() {
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

/// Execute a schedule command via the handler for a specific session.
///
/// Called by ScheduleTool::call() when the LLM invokes the tool.
/// Returns a graceful error if no handler is registered.
pub fn execute_schedule_command(session_id: Uuid, request: ScheduleRequest) -> ScheduleResult {
    let handler = match SCHEDULE_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return ScheduleResult::error("Failed to acquire schedule handlers lock");
        }
    };

    match handler {
        Some(h) => h(request),
        None => ScheduleResult::error("No schedule handler registered for this session"),
    }
}

/// Check if a schedule handler is configured for a specific session.
pub fn has_schedule_handler(session_id: Uuid) -> bool {
    SCHEDULE_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Remove all schedule handlers (test cleanup).
pub fn clear_all_schedule_handlers() {
    if let Ok(mut guard) = SCHEDULE_HANDLERS.write() {
        guard.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_has_handler() {
        let sid = Uuid::new_v4();
        assert!(!has_schedule_handler(sid));

        let handler: ScheduleHandler = Arc::new(|_req| ScheduleResult::error("stub"));
        set_schedule_handler(sid, Some(handler));
        assert!(has_schedule_handler(sid));

        set_schedule_handler(sid, None);
        assert!(!has_schedule_handler(sid));
    }

    #[test]
    fn test_execute_without_handler() {
        let sid = Uuid::new_v4();
        let req = ScheduleRequest {
            action: "list".to_string(),
            name: None,
            cron: None,
            timezone: None,
            job_type: None,
            role: None,
            prompt: None,
            command: None,
            overlap_policy: None,
        };
        let result = execute_schedule_command(sid, req);
        assert!(!result.success);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("No schedule handler registered"));
    }

    #[test]
    fn test_execute_with_handler() {
        let sid = Uuid::new_v4();
        let handler: ScheduleHandler =
            Arc::new(|req| ScheduleResult::success_remove(&req.name.unwrap_or_default()));
        set_schedule_handler(sid, Some(handler));

        let req = ScheduleRequest {
            action: "remove".to_string(),
            name: Some("test".to_string()),
            cron: None,
            timezone: None,
            job_type: None,
            role: None,
            prompt: None,
            command: None,
            overlap_policy: None,
        };
        let result = execute_schedule_command(sid, req);
        assert!(result.success);
        assert_eq!(result.name.as_deref(), Some("test"));

        // Cleanup
        set_schedule_handler(sid, None);
    }

    #[test]
    fn test_clear_all() {
        let sid1 = Uuid::new_v4();
        let sid2 = Uuid::new_v4();
        let handler: ScheduleHandler = Arc::new(|_| ScheduleResult::error("stub"));
        set_schedule_handler(sid1, Some(handler.clone()));
        set_schedule_handler(sid2, Some(handler));

        assert!(has_schedule_handler(sid1));
        assert!(has_schedule_handler(sid2));

        clear_all_schedule_handlers();

        assert!(!has_schedule_handler(sid1));
        assert!(!has_schedule_handler(sid2));
    }
}
