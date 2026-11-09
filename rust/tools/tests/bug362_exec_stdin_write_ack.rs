//! BUG-362 — a stdin write to a live unified_exec session is acknowledged
//! with its real io result, so a failed write is an error, not a success.
//!
//! Feature: spec/features/exec-stdin-write-acknowledgement.feature

use std::error::Error;

use codelet_tools::unified_exec::{
    global_store, StdinControl, UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool,
};
use rig::tool::Tool;
use serde_json::json;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn Error>>;

/// Run `command` through pipes and return its still-running exec session id.
async fn running_session(tool: &UnifiedExecTool, command: &str) -> Result<String, Box<dyn Error>> {
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(json!({
            "action": "run",
            "command": command,
            "tty": false,
            "yield_time_ms": 500
        })))
        .await?;
    let session = result.session_id.ok_or_else(|| {
        format!(
            "{command:?} exited instead of running: {:?}",
            result.exit_code
        )
    })?;
    Ok(session)
}

/// The write action's error for `input`, failing when the write succeeds.
async fn write_error(
    tool: &UnifiedExecTool,
    session: &str,
    input: &str,
) -> Result<String, Box<dyn Error>> {
    match tool
        .call(UnifiedExecArgs(json!({
            "action": "write",
            "session_id": session,
            "input": input,
            "yield_time_ms": 500
        })))
        .await
    {
        Ok(result) => {
            Err(format!("the write was reported as a success: {:?}", result.output).into())
        }
        Err(error) => Ok(error.to_string()),
    }
}

async fn close(tool: &UnifiedExecTool, session: &str) -> TestResult {
    tool.call(UnifiedExecArgs(
        json!({ "action": "close", "session_id": session }),
    ))
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writing_to_a_command_that_closed_its_stdin_returns_the_write_error() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given a running exec session whose command closed its stdin
    let tool = UnifiedExecTool::new(Uuid::nil());
    let session = running_session(&tool, "exec 0<&-; sleep 5").await?;

    // @step When the write action sends "yes" to that session
    let error = write_error(&tool, &session, "yes\n").await?;

    // @step Then the write action fails with an error naming the exec session
    assert!(
        error.contains(&session),
        "error must name {session}: {error}"
    );
    close(&tool, &session).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writing_after_the_commands_input_was_ended_returns_an_error() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given a running pipe exec session running "sleep 5"
    let tool = UnifiedExecTool::new(Uuid::nil());
    let session = running_session(&tool, "sleep 5").await?;

    // @step And its input was ended with an end-of-file control
    global_store()
        .control_stdin(&session, StdinControl::Eof)
        .await?;

    // @step When the write action sends "yes" to that session
    let error = write_error(&tool, &session, "yes\n").await?;

    // @step Then the write action fails with an error naming the exec session
    assert!(
        error.contains(&session),
        "error must name {session}: {error}"
    );
    close(&tool, &session).await
}
