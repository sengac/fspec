#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! P1 (LLM-side deterministic signal) — TOOL-022.
//!
//! Acceptance under test (feature file, P1 section):
//! - Still-running exec result carries the quiet_seconds timing fact
//! - Still-running exec result includes the fixed steering line
//! - Exited exec result carries no quiet_seconds and no steering line
//! - quiet_seconds grows as the process stays quiet
//! - quiet_seconds is a floored whole number
//!
//! Determinism contract: NO output-content inspection anywhere. The signal is
//! a timing fact (seconds since last output) plus a fixed steering line,
//! mirroring vtcode's attach_long_command_wait_steering
//! (vtcode exec_support.rs:244-258).
//!
//! These tests use a real child process via `UnifiedExecTool` (run/poll),
//! so they validate the full spawn -> reader-task -> poll path.

use codelet_tools::unified_exec::{quiet_secs_since, UnifiedExecArgs, UnifiedExecResult, STILL_RUNNING_STEERING, UnifiedExecTool};
use rig::tool::Tool;
use serde_json::json;
use uuid::Uuid;

/// Short yield window so polls return quickly; the quiet measurement is
/// wall-clock, independent of the yield window.
const YIELD_MS: u64 = 600;

async fn close_session(tool: &UnifiedExecTool, session_id: &str) {
    let _ = tool
        .call(UnifiedExecArgs(json!({
            "action": "close",
            "session_id": session_id
        })))
        .await;
}

// ============================================================================
// Scenario: Still-running exec result carries the quiet_seconds timing fact
// ============================================================================

/// Scenario: Still-running exec result carries the quiet_seconds timing fact
#[tokio::test]
async fn scenario_still_running_result_carries_quiet_seconds() {
    // @step Given a unified_exec session is running a command and has not exited
    let tool = UnifiedExecTool::new(Uuid::nil());
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "run",
            "command": "printf 'y/n: '; sleep 30",
            "yield_time_ms": YIELD_MS
        })))
        .await
        .expect("run action must succeed");
    let session_id = result
        .session_id
        .as_deref()
        .expect("still-running result must carry a session_id");

    // @step When an exec result is produced for that session
    // (the run result above IS the exec result for the still-running session)

    // @step Then the result has a session_id and no exit_code
    assert_eq!(result.exit_code, None, "still running: {result:?}");

    // @step And the result carries quiet_seconds describing how long the process has been quiet
    assert!(
        result.quiet_seconds.is_some(),
        "still-running result must carry quiet_seconds, got: {result:?}"
    );

    close_session(&tool, session_id).await;
}

// ============================================================================
// Scenario: Still-running exec result includes the fixed steering line
// ============================================================================

/// Scenario: Still-running exec result includes the fixed steering line
#[tokio::test]
async fn scenario_still_running_result_includes_steering_line() {
    // @step Given a unified_exec session is running a command and has not exited
    let tool = UnifiedExecTool::new(Uuid::nil());
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "run",
            "command": "sleep 30",
            "yield_time_ms": YIELD_MS
        })))
        .await
        .expect("run action must succeed");
    let session_id = result
        .session_id
        .as_deref()
        .expect("still-running result must carry a session_id");

    // @step When an exec result is produced for that session
    // (the run result above is the exec result)

    // @step Then the result output includes the fixed steering line
    let output = result
        .output
        .as_deref()
        .expect("still-running result must carry output");
    assert!(
        output.contains(STILL_RUNNING_STEERING),
        "output must include the steering line, got: {output:?}"
    );

    // @step And the steering line tells the LLM to send input via the write action if needed
    assert!(output.contains("write action"), "steering line text: {output:?}");

    // @step And the steering line is present regardless of what the command printed
    // `sleep 30` printed NOTHING — the steering line is still present
    // (no content inspection: the signal does not depend on output text).
    close_session(&tool, session_id).await;
}

// ============================================================================
// Scenario: Exited exec result carries no quiet_seconds and no steering line
// ============================================================================

/// Scenario: Exited exec result carries no quiet_seconds and no steering line
#[tokio::test]
async fn scenario_exited_result_has_no_quiet_seconds_or_steering() {
    // @step Given a unified_exec command printed output and then exited
    let tool = UnifiedExecTool::new(Uuid::nil());

    // @step When an exec result is produced for that session
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "run",
            "command": "echo done-tool-022",
            "yield_time_ms": YIELD_MS
        })))
        .await
        .expect("run action must succeed");

    // @step Then the result has an exit_code and no session_id
    assert_eq!(result.exit_code, Some(0), "echo must exit 0: {result:?}");
    assert_eq!(result.session_id, None);

    // @step And the result carries no quiet_seconds
    assert_eq!(
        result.quiet_seconds, None,
        "exited result must not carry quiet_seconds: {result:?}"
    );

    // @step And the result output includes no steering line
    let output = result.output.as_deref().unwrap_or("");
    assert!(
        !output.contains(STILL_RUNNING_STEERING),
        "exited result must not include the steering line: {output:?}"
    );
}

// ============================================================================
// Scenario: quiet_seconds grows as the process stays quiet
// ============================================================================

/// Scenario: quiet_seconds grows as the process stays quiet
#[tokio::test]
async fn scenario_quiet_seconds_grows_as_process_stays_quiet() {
    // @step Given a unified_exec session is running a silent command
    let tool = UnifiedExecTool::new(Uuid::nil());
    let run: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "run",
            "command": "sleep 30",
            "yield_time_ms": YIELD_MS
        })))
        .await
        .expect("run action must succeed");
    let session_id = run
        .session_id
        .as_deref()
        .expect("still-running result must carry a session_id");

    // @step When the process has been quiet for at least 1 extra second and an exec result is produced via a poll
    // (poll yield is clamped to the 5s minimum by the tool, so total
    // quiet ≈ 0.6 + 1.0 + 5.0 ≈ 6.6s, floored)
    tokio::time::sleep(std::time::Duration::from_millis(1_000)).await;
    let poll: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "poll",
            "session_id": session_id,
            "yield_time_ms": YIELD_MS
        })))
        .await
        .expect("poll action must succeed");

    // @step Then the result quiet_seconds is at least 3
    let quiet = poll.quiet_seconds.expect("poll must carry quiet_seconds");
    assert!(quiet >= 3, "quiet_seconds should be >= 3, got {quiet}: {poll:?}");

    // @step And the result quiet_seconds is at most 8
    // (upper bound with slack for the clamped 5s poll window)
    assert!(quiet <= 8, "quiet_seconds should be <= 8, got {quiet}: {poll:?}");

    close_session(&tool, session_id).await;
}

// ============================================================================
// Scenario: quiet_seconds is a floored whole number
// ============================================================================

/// Scenario: quiet_seconds is a floored whole number
#[test]
fn scenario_quiet_seconds_is_floored_whole_number() {
    // @step Given a unified_exec session is running a command and has been quiet for 4.9 seconds
    // The quiet-seconds computation is the pure floor of elapsed microseconds:
    let last_output_micros = 1_000_000u64;

    // @step When an exec result is produced for that session
    // (4.9s later)
    let now_micros = last_output_micros + 4_900_000;

    // @step Then the result quiet_seconds is 4
    assert_eq!(quiet_secs_since(last_output_micros, now_micros), 4);

    // Boundary: just under 5s still floors to 4.
    assert_eq!(
        quiet_secs_since(last_output_micros, last_output_micros + 4_999_999),
        4
    );
    // Exactly 5s floors to 5.
    assert_eq!(
        quiet_secs_since(last_output_micros, last_output_micros + 5_000_000),
        5
    );
    // No time elapsed floors to 0.
    assert_eq!(quiet_secs_since(last_output_micros, last_output_micros), 0);
}
