//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! The clock/emit (BUG-387) and fault-reporting (BUG-376) tests live in their
//! own files; shared fixtures are in test_support.rs (COV-075).

use super::test_support::*;
use super::*;

#[test]
fn test_emit_with_callback_and_clear() -> TestResult {
    let _guard = lock_tests()?;
    let session_id = Uuid::new_v4();
    let captured = capture(session_id);
    let request = request("exec-1", "git commit");
    emit_exec_stdin_request(session_id, Some(request.clone()))?;
    emit_exec_stdin_request(session_id, None)?; // BUG-171 clear
    set_exec_stdin_request_callback(session_id, None);
    assert!(emit_exec_stdin_request(session_id, Some(request.clone())).is_err());

    let events = captured.lock().map_err(|error| error.to_string())?;
    assert_eq!(
        events.len(),
        2,
        "fire + clear, then nothing after unregister"
    );
    assert_eq!(events[0], Some(request));
    assert_eq!(events[1], None, "the clear must arrive as None");
    Ok(())
}

#[test]
fn test_session_isolation() -> TestResult {
    let _guard = lock_tests()?;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let captured = capture(a);
    set_exec_stdin_request_callback(b, None);

    let request = request("exec-x", "cmd");
    // Emitted for b (no callback): reported, not delivered. For a: captured.
    assert!(emit_exec_stdin_request(b, Some(request.clone())).is_err());
    assert!(emit_exec_stdin_request(b, None).is_err());
    emit_exec_stdin_request(a, Some(request))?;

    assert_eq!(captured_len(&captured)?, 1);
    set_exec_stdin_request_callback(a, None);
    Ok(())
}

#[test]
fn test_cooldown_state_machine() -> TestResult {
    let _guard = lock_tests()?;
    // @step Given an agent session detector fired for exec session "exec-cool" 10 seconds ago
    cooldown_write().insert("exec-cool".to_string(), ago(10)?);
    // @step When the exec session is quiet again while running
    // @step Then the detector does not fire again before the 30 second cooldown elapses
    assert!(!cooldown_elapsed("exec-cool"));
    // @step Given an agent session detector fired for exec session "exec-cool" 60 seconds ago
    cooldown_write().insert("exec-cool".to_string(), ago(60)?);
    // @step When the exec session is quiet again while running
    // @step Then the detector fires again
    assert!(cooldown_elapsed("exec-cool"));
    // Never-fired session: eligible.
    assert!(cooldown_elapsed("exec-never"));
    cooldown_write().clear();
    Ok(())
}

#[test]
fn test_reset_cooldown_allows_immediate_refire() -> TestResult {
    let _guard = lock_tests()?;
    // A fire 10s ago is still inside the 30s window…
    cooldown_write().insert("exec-reset".to_string(), ago(10)?);
    assert!(
        !cooldown_elapsed("exec-reset"),
        "cooldown must still apply after a recent fire"
    );
    // …but a non-exit clear (output resumption) resets it, so a
    // fresh quiet period can fire immediately (BUG-171 rule [5]).
    reset_cooldown("exec-reset");
    assert!(
        cooldown_elapsed("exec-reset"),
        "a non-exit clear must reset the re-fire cooldown"
    );
    cooldown_write().clear();
    Ok(())
}
