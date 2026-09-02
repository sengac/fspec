#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! TOOL-022 P4 (G1/G2/G3/G4) — BashTool delegates to the unified exec
//! session machinery. The P2 quiet detector (and therefore the TUI ⌨
//! overlay) must surface for Bash commands on ANY provider, and the
//! BashTool one-shot contract (block to exit, formatted stdout/stderr +
//! exit code, ESC abort) is preserved.
//!
//! Acceptance under test (feature file, P4 section):
//! - Bash delegation surfaces the exec session steering for
//!   still-running commands (live session + piped stdin + pager
//!   suppression + detector armed + one-shot contract)
//!
//! These tests drive `BashTool` exactly as a provider's agent loop would
//! (`call`), against real child processes in the unified exec global
//! store — the same shape P1/P2 use.

use std::sync::Arc;
use std::time::Duration;

use codelet_tools::bash::BashArgs;
use codelet_tools::bash::BashTool;
use codelet_tools::unified_exec::{global_store, set_exec_stdin_request_callback, ExecStdinRequest};
use rig::tool::Tool;
use uuid::Uuid;

/// Scenario: Bash delegation surfaces the exec session steering for still-running commands
#[tokio::test]
async fn scenario_bash_delegation_surfaces_exec_stdin_prompt() {
    // @step Given an agent session on any provider runs a command via the Bash tool that will not exit quickly
    let agent = Uuid::new_v4();
    let captured = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: ExecStdinRequest| {
            captured_clone.lock().unwrap().push(request);
        })),
    );

    // Echoes the PAGER env the delegation applies (G2: pager suppression)
    // before going quiet (G4: detector must fire for ANY provider's Bash).
    // `sleep 10` outlives the detector's ~4s fire window (2s tick + 3s
    // quiet threshold) so the child is provably ALIVE when it fires.
    let tool = BashTool::new(agent);

    // @step When the Bash tool executes the command via the unified exec session machinery
    let handle = tokio::spawn(async move {
        tool
            .call(BashArgs {
                command: "sh -c 'echo PAGER=$PAGER; sleep 10'".to_string(),
                cwd: None,
            })
            .await
    });

    // The P2 detector fires when quiet >= 3s (2s tick cadence) — wait past
    // the worst-case fire time (~4s).
    tokio::time::sleep(Duration::from_millis(5_000)).await;

    // @step Then the command runs as a live unified exec session with piped stdin and pager suppression
    // (The store held the live session while the command ran — the
    // detector only fires for a store entry; the G2 pager env is
    // asserted in the one-shot contract test below.)

    // @step And the quiet detector is armed for that exec session and pushes an exec-stdin request to the agent session once the command is quiet for 3 seconds or more
    // (Copy the request out and drop the lock before the `await` below.)
    let request = {
        let requests = captured.lock().unwrap();
        assert!(
            !requests.is_empty(),
            "P2 detector must have fired for the delegated Bash session (TOOL-022 G4)"
        );
        requests[0].clone()
    };
    assert!(
        request.command.contains("sleep 10"),
        "request must carry the command display; got: {request:?}"
    );
    assert!(
        request.quiet_seconds >= 3,
        "request quiet_seconds must be >= 3; got: {request:?}"
    );
    assert!(!request.exec_session_id.is_empty());

    // G1: the per-session abort flag kills the whole delegated process
    // tree and the Bash call returns the abort error (pre-P4 contract).
    codelet_tools::request_bash_abort(agent);
    let result = handle.await.unwrap();
    assert!(result.is_err(), "abort must fail the delegated Bash call");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("interrupted by user"),
        "abort must surface the abort message"
    );

    set_exec_stdin_request_callback(agent, None);
}

/// The one-shot contract: delegated commands return the formatted
/// stdout + stderr + exit code, exactly like the pre-P4 Bash result.
#[tokio::test]
async fn bash_delegation_preserves_one_shot_result_contract() {
    // @step Given an agent session on any provider runs a command via the Bash tool that will not exit quickly
    // (fast here — the contract is about the RESULT shape, not the wait)
    let tool = BashTool::new(Uuid::new_v4());

    // @step When the Bash tool executes the command via the unified exec session machinery
    let result = tool
        .call(BashArgs {
            command: "sh -c 'echo pager=$PAGER; echo out-line; echo err-line >&2; exit 3'".to_string(),
            cwd: None,
        })
        .await;

    // @step And the Bash tool preserves its one-shot contract by blocking until the command exits, returning the formatted stdout, stderr, and exit code
    let err = result.expect_err("exit 3 must surface as a Bash error");
    let message = err.to_string();
    assert!(
        message.contains("exit code 3"),
        "result must carry the exit code; got: {message:?}"
    );
    assert!(
        message.contains("out-line"),
        "result must carry stdout; got: {message:?}"
    );
    assert!(
        message.contains("err-line"),
        "result must carry stderr; got: {message:?}"
    );
    // G2: the delegation spawn applies pager suppression to every child.
    assert!(
        message.contains("pager=cat"),
        "delegation must apply PAGER=cat (pager suppression); got: {message:?}"
    );
}

/// G1/G3: the delegated command's stdin is PIPED (not inherited) — an
/// external writer (the TUI overlay / LLM write action) can drive it.
/// The typed text flows into the running command and lands in its output.
#[tokio::test]
async fn bash_delegation_stdin_is_piped_and_writable() {
    // @step Given an agent session on any provider runs a command via the Bash tool that will not exit quickly
    let tool = BashTool::new(Uuid::new_v4());
    let handle = tokio::spawn(async move {
        tool
            .call(BashArgs {
                command: "sh -c 'read line; echo got-$line'".to_string(),
                cwd: None,
            })
            .await
    });

    // @step When the Bash tool executes the command via the unified exec session machinery
    // (let the spawn settle, then find the live exec session)
    tokio::time::sleep(Duration::from_millis(500)).await;
    let store = global_store();
    let entries = store.list_sessions().await;
    let exec_session = entries
        .into_iter()
        .find(|e| e.command.contains("read line"))
        .expect("delegated Bash command must be a live unified exec session");
    let stdin_tx = store
        .get_stdin_tx(&exec_session.session_id)
        .await
        .expect("live session must expose a piped stdin sender");

    // @step Then the command runs as a live unified exec session with piped stdin and pager suppression
    // (The stdin sender existing AND the text below landing in the
    // output proves the stdin is the session's own pipe, not inherited.)
    stdin_tx
        .send(b"p4-stdin-ok\n".to_vec())
        .await
        .expect("stdin write must succeed");

    // @step And the Bash tool preserves its one-shot contract by blocking until the command exits, returning the formatted stdout, stderr, and exit code
    let result = handle.await.unwrap();
    let output = result.expect("command must exit 0 after receiving stdin");
    assert!(
        output.contains("got-p4-stdin-ok"),
        "stdin written to the exec session must reach the command; got: {output:?}"
    );
}
