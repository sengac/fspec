//! BUG-368 — unified exec reports how a command really ended (exit code,
//! signal or unavailable) instead of sentinel values.
//!
//! Feature: spec/features/unified-exec-real-exit-status.feature

use codelet_tools::unified_exec::{ExecExit, UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool};
use rig::tool::Tool;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Run `command` and poll until it has exited.
async fn run_to_exit(
    command: &str,
    tty: bool,
) -> Result<UnifiedExecResult, Box<dyn std::error::Error>> {
    let tool = UnifiedExecTool::new(uuid::Uuid::new_v4());
    let mut result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "run", "command": command, "tty": tty, "yield_time_ms": 2000
        })))
        .await?;
    for _ in 0..10 {
        let Some(session) = result.session_id.clone() else {
            return Ok(result);
        };
        result = tool
            .call(UnifiedExecArgs(serde_json::json!({
                "action": "poll", "session_id": session, "yield_time_ms": 1000
            })))
            .await?;
    }
    Err("the command did not exit".into())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_command_killed_by_a_signal_reports_the_signal() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // BUG-447: this test drives tools without a session manager.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a unified exec command that kills itself with SIGKILL
    // @step When the command finishes
    let result = run_to_exit("kill -9 $$", false).await?;
    // @step Then the result reports a signal and no exit code
    assert_eq!(result.exit_code, None, "{result:?}");
    assert!(result.signal.is_some(), "{result:?}");
    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pty_command_reports_its_own_exit_code() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // BUG-447: this test drives tools without a session manager.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a unified exec PTY command that exits with code 3
    // @step When the command finishes
    let result = run_to_exit("sh -c 'exit 3'", true).await?;
    // @step Then the result reports exit code 3
    assert_eq!(result.exit_code, Some(3), "{result:?}");
    assert_eq!(result.signal, None, "{result:?}");
    Ok(())
}

#[test]
fn an_unavailable_exit_status_is_reported_as_unavailable() {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given an exec exit whose status could not be recovered
    let exit = ExecExit::unavailable();
    // @step When it is applied to a unified exec result
    let result = UnifiedExecResult::exited(&exit);
    // @step Then the result has no exit code and no signal and says the exit status is unavailable
    assert_eq!(result.exit_code, None);
    assert_eq!(result.signal, None);
    assert_eq!(result.exit_status_unavailable, Some(true));
}
