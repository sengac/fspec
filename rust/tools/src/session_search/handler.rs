//! SessionSearch handler mechanism
//!
//! Feature: spec/features/session-search.feature
//!
//! Provides per-session handlers for SessionSearchTool to execute searches
//! via the persistence layer in codelet-napi. Similar architecture to
//! fspec_handler.rs but for session search operations.
//!
//! ## Architecture
//!
//! 1. Session manager registers handler via `set_session_search_handler(session_id, handler)`
//! 2. SessionSearchTool (constructed with session_id) calls `execute_session_search(session_id, request)`
//! 3. Handler accesses persistence layer directly (no TypeScript round-trip)
//! 4. Handler returns SessionSearchResult to the tool
//!
//! ## Session Association (TOOL-012 pattern)
//!
//! The tool is constructed WITH its session_id. At call time, it uses
//! `self.session_id` to look up its handler — no thread-local state.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

use super::types::{SessionSearchAction, SessionSearchResult};

/// Handler function type for session search execution
/// Takes an action and the current session_id, returns the result
pub type SessionSearchHandler =
    Arc<dyn Fn(SessionSearchAction, Uuid) -> SessionSearchResult + Send + Sync>;

/// Per-session handler storage
static SESSION_SEARCH_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, SessionSearchHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the session search handler for a specific session
///
/// Called by session manager before agent run to configure how session
/// searches are executed for this session.
pub fn set_session_search_handler(session_id: Uuid, handler: Option<SessionSearchHandler>) {
    if let Ok(mut guard) = SESSION_SEARCH_HANDLERS.write() {
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

/// Check if a session search handler is configured for a specific session
pub fn has_session_search_handler(session_id: Uuid) -> bool {
    SESSION_SEARCH_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute a session search action via the handler for a specific session
///
/// Called by SessionSearchTool when the LLM invokes the tool.
pub fn execute_session_search(
    session_id: Uuid,
    action: SessionSearchAction,
) -> SessionSearchResult {
    let handler = match SESSION_SEARCH_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return SessionSearchResult::Error {
                message: "Failed to acquire session search handlers lock".to_string(),
            };
        }
    };

    match handler {
        Some(h) => h(action, session_id),
        None => SessionSearchResult::Error {
            message: format!(
                "Session search handler not configured for session {session_id} — \
                 SessionSearchTool requires session context"
            ),
        },
    }
}

/// Clear all session search handlers (for testing)
pub fn clear_all_session_search_handlers() {
    if let Ok(mut guard) = SESSION_SEARCH_HANDLERS.write() {
        guard.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn with_clean_handlers<T>(f: impl FnOnce() -> T) -> T {
        clear_all_session_search_handlers();
        let result = f();
        clear_all_session_search_handlers();
        result
    }

    /// Scenario: SessionSearch uses persistence layer directly
    // @step Given the SessionSearch tool is compiled as native Rust
    // @step When any SessionSearch action is invoked
    // @step Then data is read from MessageStore, HistoryStore, and BlobStore directly
    // @step And no Python or bash subprocess is spawned
    #[test]
    #[serial]
    fn test_no_handler_returns_error() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();
            let result =
                execute_session_search(session_id, SessionSearchAction::Recent { count: Some(5) });

            match result {
                SessionSearchResult::Error { message } => {
                    assert!(message.contains("not configured"));
                }
                _ => panic!("Expected Error result when no handler configured"),
            }
        });
    }

    /// Scenario: Show non-existent session returns error
    // @step Given no session exists with ID "00000000-0000-0000-0000-000000000000"
    // @step When the agent calls SessionSearch with action "show" and session_id "00000000-0000-0000-0000-000000000000"
    // @step Then the tool returns an error indicating the session was not found
    #[test]
    #[serial]
    fn test_handler_receives_action() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();
            let called = Arc::new(AtomicBool::new(false));
            let called_clone = called.clone();

            let handler: SessionSearchHandler = Arc::new(move |action, _sid| {
                called_clone.store(true, Ordering::SeqCst);
                match action {
                    SessionSearchAction::Recent { count } => {
                        assert_eq!(count, Some(5));
                        SessionSearchResult::Recent { sessions: vec![] }
                    }
                    _ => SessionSearchResult::Error {
                        message: "unexpected action".to_string(),
                    },
                }
            });

            set_session_search_handler(session_id, Some(handler));

            let result =
                execute_session_search(session_id, SessionSearchAction::Recent { count: Some(5) });

            assert!(called.load(Ordering::SeqCst));
            match result {
                SessionSearchResult::Recent { sessions } => {
                    assert!(sessions.is_empty());
                }
                _ => panic!("Expected Recent result"),
            }
        });
    }

    #[test]
    #[serial]
    fn test_has_session_search_handler() {
        with_clean_handlers(|| {
            let session_id = Uuid::new_v4();

            assert!(!has_session_search_handler(session_id));

            let handler: SessionSearchHandler = Arc::new(|_, _| SessionSearchResult::Error {
                message: "stub".to_string(),
            });
            set_session_search_handler(session_id, Some(handler));

            assert!(has_session_search_handler(session_id));

            set_session_search_handler(session_id, None);
            assert!(!has_session_search_handler(session_id));
        });
    }

    #[test]
    #[serial]
    fn test_concurrent_sessions_isolated() {
        with_clean_handlers(|| {
            let session_a = Uuid::new_v4();
            let session_b = Uuid::new_v4();

            let handler_a: SessionSearchHandler =
                Arc::new(|_, _| SessionSearchResult::Recent { sessions: vec![] });
            set_session_search_handler(session_a, Some(handler_a));

            let handler_b: SessionSearchHandler = Arc::new(|_, _| SessionSearchResult::Error {
                message: "from_b".to_string(),
            });
            set_session_search_handler(session_b, Some(handler_b));

            // session_a returns Recent
            let result_a =
                execute_session_search(session_a, SessionSearchAction::Recent { count: None });
            match result_a {
                SessionSearchResult::Recent { .. } => {}
                _ => panic!("Expected Recent from session_a"),
            }

            // session_b returns Error
            let result_b =
                execute_session_search(session_b, SessionSearchAction::Recent { count: None });
            match result_b {
                SessionSearchResult::Error { message } => {
                    assert_eq!(message, "from_b");
                }
                _ => panic!("Expected Error from session_b"),
            }

            // Remove session_b handler — session_a still works
            set_session_search_handler(session_b, None);
            let result_a2 =
                execute_session_search(session_a, SessionSearchAction::Recent { count: None });
            match result_a2 {
                SessionSearchResult::Recent { .. } => {}
                _ => panic!("Expected Recent from session_a after removing b"),
            }
        });
    }
}
