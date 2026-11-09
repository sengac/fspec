//! Feature: spec/features/exec-stdin-clock-and-emit-errors.feature
//!
//! BUG-387 tests, moved out of tests.rs so each test file maps one feature (COV-075).

use super::test_support::*;
use super::*;
use std::time::UNIX_EPOCH;

#[test]
fn a_request_timestamp_before_the_epoch_is_an_error() -> TestResult {
    // @step Given a system clock reading before the Unix epoch
    let before = UNIX_EPOCH
        .checked_sub(Duration::from_secs(1))
        .ok_or("the platform cannot represent a pre-epoch time")?;
    // @step When the exec-stdin request timestamp is taken
    let stamp = epoch_ms(before);
    // @step Then it is an error instead of 0
    if stamp.is_ok() {
        return Err(format!("a pre-epoch clock produced {stamp:?}"));
    }
    Ok(())
}

#[test]
fn the_refire_cooldown_runs_on_the_monotonic_clock() -> TestResult {
    // @step Given a fire recorded on the monotonic clock
    let fired = ago(0)?;
    // @step When 29 seconds and then 30 seconds have passed
    let at_29 = cooldown_has_elapsed(fired, fired + Duration::from_secs(29));
    let at_30 = cooldown_has_elapsed(fired, fired + Duration::from_secs(30));
    // @step Then the cooldown holds at 29 seconds and has elapsed at 30 seconds
    assert!(!at_29, "the cooldown must hold at 29s");
    assert!(at_30, "the cooldown must elapse at 30s");
    Ok(())
}

#[test]
fn emitting_to_a_session_without_a_callback_is_an_error() -> TestResult {
    let _guard = lock_tests()?;
    // @step Given an agent session with no exec-stdin request callback
    let session_id = Uuid::new_v4();
    set_exec_stdin_request_callback(session_id, None);
    // @step When an exec-stdin request is emitted for it
    let fire = emit_exec_stdin_request(session_id, Some(request("exec-1", "cmd")));
    let clear = emit_exec_stdin_request(session_id, None);
    // @step Then the emit fails naming the agent session
    for result in [fire, clear] {
        let error = result.err().unwrap_or_default();
        assert!(error.contains(&session_id.to_string()), "{error}");
    }
    Ok(())
}
