#![doc = "Feature: spec/features/transcript-admission-and-model-separation.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent};
use codelet_core::persistence::transcript::{JournalEvent, JournalStore, TerminalOutcome};
use std::collections::HashSet;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn transcript_outcomes_never_enter_model_originals() -> TestResult {
    // @step Given a session with assistant originals and transcript-only terminal records
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Text("authentic partial".into()));
    output.emit(StreamEvent::Error(
        "Bearer sk-secret prompt=private tool_payload=private".into(),
    ));
    let originals = read(session.id)?;
    // @step When the next model history is restored
    // @step Then terminal prose is absent from model input and original coordinates are unchanged
    assert_eq!(originals.len(), 1);
    assert_eq!(
        originals[0]["message"]["content"][0]["text"],
        "authentic partial"
    );
    let ids = originals
        .iter()
        .filter_map(|record| record["uuid"].as_str()?.parse().ok())
        .collect();
    let projection = JournalStore::new(directory.path(), session.id).project(&ids)?;
    assert_eq!(projection.records.len(), 1);
    assert!(matches!(
        projection.records[0].event,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError
        }
    ));
    let bytes = serde_json::to_string(&projection.records)?;
    assert!(!bytes.contains("sk-secret"));
    assert!(!bytes.contains("private"));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn transcript_persistence_failure_blocks_publication_and_dispatch() -> TestResult {
    // @step Given an unavailable durable transcript directory
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    std::fs::write(directory.path().join("transcripts"), "unavailable")?;
    let mut events = session.subscribe_to_stream();
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Text("not acknowledged".into()));
    // @step When a visible prefix or tool invocation needs acknowledgement
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "blocked".into(),
        call_id: None,
        name: "write".into(),
        args: serde_json::json!({}),
    }));
    assert!(output.prepare_tool_dispatch("blocked").is_err());
    // @step Then no prefix is published and no tool is executed
    let mut seen = Vec::new();
    while let Ok(event) = events.try_recv() {
        seen.push(event);
    }
    assert!(!seen.iter().any(|event| matches!(
        event,
        codelet_rpc_types::StreamChunk::Text { .. }
            | codelet_rpc_types::StreamChunk::ToolCall { .. }
    )));
    std::fs::remove_file(directory.path().join("transcripts"))?;
    codelet_agent_loop::persist::retry_pending_persistence(&session.id)?;
    // @step And the original record remains with the shared bounded recovery owner
    let projection = JournalStore::new(directory.path(), session.id).project(&HashSet::new())?;
    assert!(projection.records.iter().any(
        |record| matches!(&record.event, JournalEvent::Text { text } if text == "not acknowledged")
    ));
    assert!(projection.tools.is_empty());
    Ok(())
}
