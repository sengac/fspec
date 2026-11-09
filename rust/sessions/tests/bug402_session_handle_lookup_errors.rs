//! BUG-402 — Result-returning session handle methods report the real
//! session lookup error.
//!
//! Feature: spec/features/session-handle-lookup-errors.feature

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{HumanActionReply, SessionId, ThinkingLevel};
use codelet_sessions::session_manager::SessionManager;

/// Every Result-returning lookup through the handle, for `sid`.
fn lookups(handle: &SessionManager, sid: &SessionId) -> Vec<(&'static str, Result<(), String>)> {
    vec![
        (
            "pause_human_action",
            SessionManagerHandle::pause_human_action(
                handle,
                sid,
                "request-1",
                HumanActionReply::Done,
            ),
        ),
        (
            "pause_confirm",
            SessionManagerHandle::pause_confirm(handle, sid, "request-1", true),
        ),
        (
            "clear_history",
            SessionManagerHandle::clear_history(handle, sid),
        ),
        (
            "set_thinking_level",
            SessionManagerHandle::set_thinking_level(
                handle,
                sid,
                ThinkingLevel::Effort("low".to_string()),
            ),
        ),
        (
            "set_continue_state",
            SessionManagerHandle::set_continue_state(handle, sid, true, 1),
        ),
        (
            "destroy_session",
            SessionManagerHandle::destroy_session(handle, sid),
        ),
        (
            "get_pause_state",
            SessionManagerHandle::get_pause_state(handle, sid).map(|_| ()),
        ),
    ]
}

fn assert_all_fail_with(results: Vec<(&'static str, Result<(), String>)>, expected: &str) {
    for (method, result) in results {
        let error = result.err().unwrap_or_default();
        assert!(
            error.contains(expected),
            "{method}: expected an error containing `{expected}`, got `{error}`"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_session_id_is_reported_as_invalid() {
    // @step Given a session manager handle
    let manager = SessionManager::new();
    // @step When session operations are called with a malformed session id
    let results = lookups(&manager, &SessionId::new("bad-id".to_string()));
    // @step Then each fails with an invalid session ID error
    assert_all_fail_with(results, "Invalid session ID");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_session_is_still_reported_as_not_found() {
    // @step Given a session manager handle
    let manager = SessionManager::new();
    // @step When session operations are called with a well-formed id of no session
    let results = lookups(&manager, &SessionId::new(uuid::Uuid::new_v4().to_string()));
    // @step Then each fails with a session not found error
    assert_all_fail_with(results, "Session not found");
}
