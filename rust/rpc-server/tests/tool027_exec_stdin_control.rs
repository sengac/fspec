//! TOOL-027: send an interrupt or end of input to a running exec session.
//!
//! Feature: spec/features/exec-stdin-control.feature
//!
//! A real daemon over WebSocket with a real SessionManager. Exec sessions
//! start in-process, so they share the daemon's unified_exec store.

mod tool027_exec_stdin_control_support;

use codelet_rpc_types::{ExecStdinControl, SessionId, SessionStatus};
use codelet_tools::unified_exec::ExecStdinRequest;
use tarpc::context;
use tool027_exec_stdin_control_support::*;

async fn control(
    daemon: &Daemon,
    session: &SessionId,
    exec_id: &str,
    control: ExecStdinControl,
) -> Boxed<Result<(), String>> {
    let client = daemon.client().await?;
    Ok(client
        .client()
        .control_exec_stdin(
            context::current(),
            session.clone(),
            exec_id.to_string(),
            control,
        )
        .await?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupt_sends_sigint_to_a_pipe_exec_sessions_command() -> TestResult {
    // @step Given a daemon client and an agent session with a live pipe exec session that prints "got-int" on SIGINT
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;
    let exec = start_exec(&session, TRAP_SCRIPT, false).await?;

    // @step When the client sends an interrupt to that exec session over the RPC
    control(&daemon, &session, &exec.id, ExecStdinControl::Interrupt).await??;

    // @step Then the exec session prints "got-int" and exits
    let exited = wait_for_exit(&exec).await?;
    assert!(
        exited.output.contains("got-int"),
        "output: {}",
        exited.output
    );
    assert_eq!(exited.exit_code, Some(3));

    // @step And the agent session is still idle
    let client = daemon.client().await?;
    let status = client
        .client()
        .get_session_status(context::current(), session)
        .await??;
    assert_eq!(status, SessionStatus::Idle);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupt_reaches_a_pty_exec_sessions_foreground_command() -> TestResult {
    // @step Given a daemon client and an agent session with a live PTY exec session that prints "got-int" on SIGINT
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;
    let exec = start_exec(&session, TRAP_SCRIPT, true).await?;

    // @step When the client sends an interrupt to that exec session over the RPC
    control(&daemon, &session, &exec.id, ExecStdinControl::Interrupt).await??;

    // @step Then the exec session prints "got-int" and exits
    let exited = wait_for_exit(&exec).await?;
    assert!(
        exited.output.contains("got-int"),
        "output: {}",
        exited.output
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn end_of_input_closes_a_pipe_exec_sessions_stdin() -> TestResult {
    // @step Given a daemon client and an agent session with a live pipe exec session running "cat"
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;
    let exec = start_exec(&session, "cat", false).await?;

    // @step When the client sends end of input to that exec session over the RPC
    control(&daemon, &session, &exec.id, ExecStdinControl::Eof).await??;

    // @step Then the exec session exits with status 0
    let exited = wait_for_exit(&exec).await?;
    assert_eq!(exited.exit_code, Some(0), "output: {}", exited.output);

    // @step And a later write to that exec session fails with an error naming the exec session
    let client = daemon.client().await?;
    let write = client
        .client()
        .write_exec_stdin(
            context::current(),
            session,
            exec.id.clone(),
            "late".to_string(),
        )
        .await?;
    let error = write.err().ok_or("a write after end of input must fail")?;
    assert!(error.contains(&exec.id), "error: {error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn end_of_input_ends_a_pty_exec_sessions_input() -> TestResult {
    // @step Given a daemon client and an agent session with a live PTY exec session running "cat"
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;
    let exec = start_exec(&session, "cat", true).await?;

    // @step When the client sends end of input to that exec session over the RPC
    control(&daemon, &session, &exec.id, ExecStdinControl::Eof).await??;

    // @step Then the exec session exits
    wait_for_exit(&exec).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_successful_control_clears_the_pending_exec_stdin_request() -> TestResult {
    // @step Given a daemon client and an agent session with a pending exec-stdin request for a live pipe exec session running "cat"
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;
    let exec = start_exec(&session, "cat", false).await?;
    daemon
        .manager
        .get_session(&session.value)?
        .set_exec_stdin_request(Some(ExecStdinRequest {
            exec_session_id: exec.id.clone(),
            command: "cat".to_string(),
            quiet_seconds: 5,
            ts_ms: 1_700_000_000_000,
        }))?;
    let client = daemon.client().await?;
    let pending = client
        .client()
        .get_exec_stdin_request(context::current(), session.clone())
        .await??;
    assert!(
        pending.is_some(),
        "the request must be pending before the control"
    );

    // @step When the client sends end of input to that exec session over the RPC
    control(&daemon, &session, &exec.id, ExecStdinControl::Eof).await??;

    // @step Then the agent session has no exec-stdin request
    let after = client
        .client()
        .get_exec_stdin_request(context::current(), session)
        .await??;
    assert_eq!(after, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn control_on_an_unknown_exec_session_or_agent_session_fails_with_a_clean_error() -> TestResult
{
    // @step Given a daemon client and an agent session with no exec session "exec-missing"
    let daemon = Daemon::start().await?;
    let session = daemon.session().await?;

    // @step When the client sends an interrupt to exec session "exec-missing" over the RPC
    let missing = control(
        &daemon,
        &session,
        "exec-missing",
        ExecStdinControl::Interrupt,
    )
    .await?;

    // @step Then the call fails with an error naming "exec-missing"
    let error = missing.err().ok_or("an unknown exec session must fail")?;
    assert!(error.contains("exec-missing"), "error: {error}");

    // @step And an interrupt for an unknown agent session fails with "Session not found"
    let stranger = SessionId {
        value: uuid::Uuid::new_v4().to_string(),
    };
    let unknown = control(
        &daemon,
        &stranger,
        "exec-missing",
        ExecStdinControl::Interrupt,
    )
    .await?;
    let error = unknown.err().ok_or("an unknown agent session must fail")?;
    assert!(error.contains("Session not found"), "error: {error}");
    Ok(())
}

#[test]
fn control_exec_stdin_is_appended_after_every_existing_fspecservice_method() -> TestResult {
    // @step Given the FspecService methods in declaration order
    let methods = declared_methods()?;

    // @step When the index of control_exec_stdin is read
    let index = methods.iter().position(|name| name == "control_exec_stdin");

    // @step Then control_exec_stdin keeps index 130, after every method declared before it
    assert_eq!(index, Some(130));
    let frozen = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rpc/tests/fixtures/wire_order_frozen.txt"),
    )?;
    let listed: Vec<&str> = frozen
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(listed.get(130).copied(), Some("control_exec_stdin"));
    Ok(())
}
