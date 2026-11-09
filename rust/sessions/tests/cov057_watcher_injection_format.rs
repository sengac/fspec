//! Feature: spec/features/watcher-injection-message-format.feature
//!
//! COV-057: WATCH-006's `WatcherInput` / `format_watcher_input` are now
//! `codelet_sessions::background_session::IncomingMessage` /
//! `format_incoming_message`, whose prefix is
//! `[SUPERVISOR: <role> | Session: <id>]` (the former `Authority` field no
//! longer exists). The delivery scenario runs the real
//! `codelet_agent_loop::agent_loop` over a stub-provider session (offline)
//! and observes the `StreamChunk::IncomingMessage` it broadcasts.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "cov057_support/session.rs"]
pub mod support;

use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{format_incoming_message, IncomingMessage};
use support::{data_directory, stub_session, TestResult};

#[test]
fn scenario_format_peer_watcher_message_with_structured_prefix() -> TestResult {
    // @step Given a watcher session with role "code-reviewer"
    let role = "code-reviewer";
    // @step And the watcher session id is "abc123"
    let session_id = "abc123";
    // @step When the watcher sends message "Consider adding error handling"
    let input = IncomingMessage::new(
        session_id.into(),
        role.into(),
        "Consider adding error handling".into(),
    )?;
    // @step Then the formatted message should be "[SUPERVISOR: code-reviewer | Session: abc123] Consider adding error handling"
    assert_eq!(
        format_incoming_message(&input),
        "[SUPERVISOR: code-reviewer | Session: abc123] Consider adding error handling"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_format_supervisor_watcher_message_with_structured_prefix() -> TestResult {
    let directory = data_directory()?;
    codelet_providers::stub_provider::register_stub_provider();
    let (parent, receiver) = stub_session(directory.path(), "parent")?;
    let mut events = parent.subscribe_to_stream();
    // @step Given a watcher session with role "security-auditor"
    let role = "security-auditor";
    // @step And the watcher session id is "xyz789"
    let watcher_id = "xyz789";
    // @step When the watcher sends message "CRITICAL: SQL injection vulnerability detected"
    parent.receive_incoming_message(IncomingMessage::new(
        watcher_id.into(),
        role.into(),
        "CRITICAL: SQL injection vulnerability detected".into(),
    )?)?;
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        parent.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    let delivered = codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            if let StreamChunk::IncomingMessage { text, .. } = events.recv().await? {
                return Ok::<String, Box<dyn std::error::Error>>(text);
            }
        }
    })
    .await;
    worker.abort();
    let _ = worker.await;
    // @step Then the parent should receive an IncomingMessage chunk
    let text = delivered??;
    // @step And the chunk should contain the formatted message with structured prefix
    assert_eq!(
        text,
        "[SUPERVISOR: security-auditor | Session: xyz789] CRITICAL: SQL injection vulnerability detected"
    );
    Ok(())
}

#[test]
fn scenario_empty_watcher_message_returns_error() {
    // @step Given a watcher session with role "test-watcher"
    let role = "test-watcher";
    // @step And the watcher session id is "test123"
    let session_id = "test123";
    // @step When the watcher sends an empty message
    let result = IncomingMessage::new(session_id.into(), role.into(), String::new());
    // @step Then an error should be returned with message "message cannot be empty"
    assert_eq!(result.err().as_deref(), Some("message cannot be empty"));
}

#[test]
fn scenario_multiline_watcher_message_preserves_formatting() -> TestResult {
    // @step Given a watcher session with role "code-reviewer"
    let role = "code-reviewer";
    // @step And the watcher session id is "abc123"
    let session_id = "abc123";
    // @step When the watcher sends a multiline message:
    let message = "Issue found on line 42:\n- Missing null check\n- Consider using Option<T>";
    let input = IncomingMessage::new(session_id.into(), role.into(), message.into())?;
    let formatted = format_incoming_message(&input);
    let lines: Vec<&str> = formatted.lines().collect();
    // @step Then the formatted message should have the prefix on the first line
    assert_eq!(
        lines.first().copied(),
        Some("[SUPERVISOR: code-reviewer | Session: abc123] Issue found on line 42:")
    );
    // @step And subsequent lines should be preserved without additional prefixes
    assert_eq!(
        lines.get(1..),
        Some(&["- Missing null check", "- Consider using Option<T>"][..])
    );
    assert_eq!(formatted.matches("[SUPERVISOR:").count(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_receive_watcher_input_queues_message_asynchronously() -> TestResult {
    let directory = data_directory()?;
    // @step Given a parent session exists
    let (parent, _receiver) = stub_session(directory.path(), "parent")?;
    let input = IncomingMessage::new(
        "w1".into(),
        "reviewer".into(),
        "Check the error path".into(),
    )?;
    // @step When receive_watcher_input is called with a valid WatcherInput
    let result = parent.receive_incoming_message(input);
    result?;
    // @step Then the input should be queued via the watcher input channel
    assert_eq!(parent.pending_incoming_message_count(), 1);
    let queued = parent.incoming_message_rx.lock().await.try_recv()?;
    assert_eq!(
        (queued.source_session_id.as_str(), queued.role_name.as_str()),
        ("w1", "reviewer")
    );
    assert_eq!(queued.message, "Check the error path");
    assert!(queued.submission_id.is_some(), "durable receipt retained");
    // @step And the method should return immediately without blocking
    // No agent loop is consuming the channel; the call returned on its own
    // (TEST-042: returning at all is the proof, not a time bound).
    Ok(())
}
