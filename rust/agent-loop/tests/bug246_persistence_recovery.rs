#![doc = "Feature: spec/features/recoverable-conversation-persistence.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use rig::tool::Tool;
use serde_json::json;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn assistant_content_survives_a_temporary_archive_failure() -> TestResult {
    // @step Given a real session with an unavailable message archive
    let (directory, session, _) = fixture()?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    // @step When assistant text settles while the archive is unavailable
    output.emit(StreamEvent::Text("original prefix α".into()));
    output.emit(StreamEvent::Done(Some("end_turn".into())));
    drop(output);
    // @step And the archive is repaired and persistence is retried
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Done(Some("end_turn".into())));
    // @step Then cold reopen contains the original assistant text exactly once
    let envelopes = read(session.id)?;
    assert_eq!(envelopes.len(), 1);
    assert_eq!(
        envelopes[0]["message"]["content"][0]["text"],
        "original prefix α"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn an_invocation_failure_cannot_orphan_its_authentic_result() -> TestResult {
    // @step Given a real session with an unavailable message archive
    let (directory, session, _) = fixture()?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    // @step When a tool invocation and its authentic result settle
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "invocation".into(),
        call_id: Some("native-call".into()),
        name: "inspect".into(),
        args: json!({}),
    }));
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: "invocation".into(),
        call_id: Some("native-call".into()),
        content: "authentic output".into(),
        is_error: false,
    }));
    // @step And the archive is repaired and persistence is retried
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    output.emit(StreamEvent::Done(Some("end_turn".into())));
    // @step Then cold reopen contains the invocation before its matching result exactly once
    let envelopes = read(session.id)?;
    assert_eq!(envelopes.len(), 2);
    assert_eq!(envelopes[0]["message"]["content"][0]["id"], "invocation");
    assert_eq!(
        envelopes[1]["message"]["content"][0]["tool_use_id"],
        "invocation"
    );
    assert_eq!(
        envelopes[1]["message"]["content"][0]["content"],
        "authentic output"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn retrying_a_captured_result_never_repeats_its_side_effect() -> TestResult {
    // @step Given an archived invocation for a sentinel writing tool
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "sentinel".into(),
        call_id: None,
        name: "write".into(),
        args: json!({}),
    }));
    output.emit(StreamEvent::Done(None));
    let authentic = codelet_tools::bash::BashTool::new(session.id)
        .call(codelet_tools::bash::BashArgs {
            command: "printf 'one external action\\n' >> sentinel; printf 'captured final result'"
                .into(),
            cwd: Some(directory.path().to_string_lossy().into()),
        })
        .await?;
    // @step When its captured result cannot be archived
    std::fs::rename(
        directory.path().join("messages/messages.jsonl"),
        directory.path().join("messages/saved"),
    )?;
    std::fs::create_dir(directory.path().join("messages/messages.jsonl"))?;
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: "sentinel".into(),
        call_id: None,
        content: authentic.clone(),
        is_error: false,
    }));
    // @step And the archive is repaired and persistence is retried
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    std::fs::rename(
        directory.path().join("messages/saved"),
        directory.path().join("messages/messages.jsonl"),
    )?;
    output.emit(StreamEvent::Done(None));
    output.emit(StreamEvent::Done(None));
    // @step Then the authentic result is archived once and the sentinel action ran once
    let envelopes = read(session.id)?;
    let captured: Vec<_> = envelopes
        .iter()
        .filter(|record| record["message"]["content"][0]["content"] == authentic)
        .collect();
    assert_eq!(captured.len(), 1);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("sentinel"))?,
        "one external action\n"
    );
    Ok(())
}
