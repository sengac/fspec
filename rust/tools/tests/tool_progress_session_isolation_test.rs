#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/tool-progress-session-isolation.feature
//!
//! Tests for BUG-126: TOOL_PROGRESS_CALLBACK per-session isolation.
//!
//! These tests verify that tool progress callbacks are keyed by session_id
//! so that concurrent sessions never leak output to each other.
//!
//! Test order matches the feature file scenario order for traceability.

use codelet_tools::tool_progress::{emit_tool_progress, set_tool_progress_callback};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Test lock to ensure tests that mutate global state run sequentially.
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ============================================================================
// Scenario 1: Per-session callback isolation — emit dispatches only to the
//             registered session
// ============================================================================

#[test]
fn test_per_session_callback_isolation() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    let captured_a = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));
    let captured_b = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));

    // @step Given session A has registered a tool progress callback
    let cap_a = captured_a.clone();
    set_tool_progress_callback(
        session_a,
        Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            cap_a.lock().unwrap().push((chunk.to_string(), is_stderr));
        })),
    );

    // @step And session B has registered a different tool progress callback
    let cap_b = captured_b.clone();
    set_tool_progress_callback(
        session_b,
        Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            cap_b.lock().unwrap().push((chunk.to_string(), is_stderr));
        })),
    );

    // @step When tool progress is emitted for session A
    emit_tool_progress(session_a, "hello from A\n", false);

    // @step Then only session A's callback is invoked
    let events_a = captured_a.lock().unwrap();
    assert_eq!(events_a.len(), 1);
    assert_eq!(events_a[0].0, "hello from A\n");

    // @step And session B's callback is not invoked
    let events_b = captured_b.lock().unwrap();
    assert_eq!(events_b.len(), 0);

    // Cleanup
    set_tool_progress_callback(session_a, None);
    set_tool_progress_callback(session_b, None);
}

// ============================================================================
// Scenario 2: Clearing one session's callback does not affect another session
// ============================================================================

#[test]
fn test_clearing_one_session_does_not_affect_another() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    let captured_a = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));
    let captured_b = Arc::new(Mutex::new(Vec::<(String, bool)>::new()));

    // @step Given session A has registered a tool progress callback
    let cap_a = captured_a.clone();
    set_tool_progress_callback(
        session_a,
        Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            cap_a.lock().unwrap().push((chunk.to_string(), is_stderr));
        })),
    );

    // @step And session B has registered a tool progress callback
    let cap_b = captured_b.clone();
    set_tool_progress_callback(
        session_b,
        Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            cap_b.lock().unwrap().push((chunk.to_string(), is_stderr));
        })),
    );

    // @step When session B's callback is cleared
    set_tool_progress_callback(session_b, None);

    // @step And tool progress is emitted for session A
    emit_tool_progress(session_a, "still working\n", false);

    // @step Then session A's callback is invoked normally
    let events_a = captured_a.lock().unwrap();
    assert_eq!(events_a.len(), 1);
    assert_eq!(events_a[0].0, "still working\n");

    // @step And emitting tool progress for session B is a no-op
    drop(events_a); // release lock before emitting
    emit_tool_progress(session_b, "should go nowhere\n", false);
    let events_b = captured_b.lock().unwrap();
    assert_eq!(events_b.len(), 0);

    // Cleanup
    set_tool_progress_callback(session_a, None);
}

// ============================================================================
// Scenario 3: Emitting tool progress for an unregistered session is a silent
//             no-op
// ============================================================================

#[test]
fn test_emit_for_unregistered_session_is_noop() {
    let _guard = TEST_LOCK.lock().unwrap();

    // @step Given no callback is registered for session C
    let session_c = Uuid::new_v4();
    // (deliberately do NOT register anything)

    // @step When tool progress is emitted for session C
    // @step Then no callback is invoked
    // @step And no error or panic occurs
    emit_tool_progress(session_c, "orphan output\n", false);
    emit_tool_progress(session_c, "orphan stderr\n", true);
    // If we reach here without panicking, the test passes.
}

// ============================================================================
// Scenario 4: Multiple concurrent callbacks operate independently
// ============================================================================

#[test]
fn test_multiple_concurrent_callbacks_independent() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_a = Uuid::new_v4();
    let session_b = Uuid::new_v4();

    // @step Given session A has registered a tool progress callback capturing output to buffer A
    let buffer_a = Arc::new(Mutex::new(Vec::<String>::new()));
    let buf_a = buffer_a.clone();
    set_tool_progress_callback(
        session_a,
        Some(Arc::new(move |chunk: &str, _is_stderr: bool| {
            buf_a.lock().unwrap().push(chunk.to_string());
        })),
    );

    // @step And session B has registered a tool progress callback capturing output to buffer B
    let buffer_b = Arc::new(Mutex::new(Vec::<String>::new()));
    let buf_b = buffer_b.clone();
    set_tool_progress_callback(
        session_b,
        Some(Arc::new(move |chunk: &str, _is_stderr: bool| {
            buf_b.lock().unwrap().push(chunk.to_string());
        })),
    );

    // @step When tool progress "stdout line A" is emitted for session A
    emit_tool_progress(session_a, "stdout line A", false);

    // @step And tool progress "stdout line B" is emitted for session B
    emit_tool_progress(session_b, "stdout line B", false);

    // @step Then buffer A contains only "stdout line A"
    let a_contents = buffer_a.lock().unwrap();
    assert_eq!(a_contents.len(), 1);
    assert_eq!(a_contents[0], "stdout line A");

    // @step And buffer B contains only "stdout line B"
    let b_contents = buffer_b.lock().unwrap();
    assert_eq!(b_contents.len(), 1);
    assert_eq!(b_contents[0], "stdout line B");

    // Cleanup
    drop(a_contents);
    drop(b_contents);
    set_tool_progress_callback(session_a, None);
    set_tool_progress_callback(session_b, None);
}

// ============================================================================
// Scenario 5: Registering and clearing callbacks for many sessions concurrently
// ============================================================================

#[test]
fn test_many_sessions_concurrent_registration() {
    let _guard = TEST_LOCK.lock().unwrap();

    let session_count = 10;
    let sessions: Vec<Uuid> = (0..session_count).map(|_| Uuid::new_v4()).collect();
    let buffers: Vec<Arc<Mutex<Vec<String>>>> = (0..session_count)
        .map(|_| Arc::new(Mutex::new(Vec::new())))
        .collect();

    // @step Given 10 sessions have each registered a tool progress callback
    for i in 0..session_count {
        let buf = buffers[i].clone();
        set_tool_progress_callback(
            sessions[i],
            Some(Arc::new(move |chunk: &str, _: bool| {
                buf.lock().unwrap().push(chunk.to_string());
            })),
        );
    }

    // @step When tool progress is emitted for each session with a unique message
    for (i, &session) in sessions.iter().enumerate().take(session_count) {
        emit_tool_progress(session, &format!("msg-{i}"), false);
    }

    // @step Then each session's callback received only its own message
    for (i, buffer) in buffers.iter().enumerate().take(session_count) {
        let contents = buffer.lock().unwrap();
        assert_eq!(
            contents.len(),
            1,
            "Session {i} should have exactly 1 message"
        );
        assert_eq!(contents[0], format!("msg-{i}"));
    }

    // @step And clearing all callbacks leaves the registry empty
    for &session in sessions.iter().take(session_count) {
        set_tool_progress_callback(session, None);
    }

    // Verify all are cleared — emit should be no-ops
    for (i, (&session, buffer)) in sessions
        .iter()
        .zip(buffers.iter())
        .enumerate()
        .take(session_count)
    {
        emit_tool_progress(session, "should not arrive", false);
        let contents = buffer.lock().unwrap();
        assert_eq!(
            contents.len(),
            1,
            "Session {i} should still have only its 1 original message after clear"
        );
    }
}
