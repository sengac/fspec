#![doc = "Feature: spec/features/persistence-result-capacity.feature"]

use codelet_agent_loop::{
    persist::retry_pending_persistence,
    persistence_retry::{PersistenceSpool, RecoveryRecord, MAX_PENDING_RECORDS},
    BackgroundOutput,
};
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use codelet_core::persistence::transcript::{JournalEvent, JournalStore};
use codelet_rpc_types::StreamChunk;
use rig::tool::Tool;
use serde_json::json;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn captured_tool_result_survives_exhausted_recovery_capacity() -> TestResult {
    // @step Given a dispatched sentinel tool and a session at recovery admission capacity
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let mut events = session.subscribe_to_stream();
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "sentinel".into(),
        call_id: Some("native".into()),
        name: "write".into(),
        args: json!({}),
    }));
    output.prepare_tool_dispatch("sentinel")?;
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "queued".into(),
        call_id: None,
        name: "write".into(),
        args: json!({}),
    }));
    let authentic = codelet_tools::bash::BashTool::new(session.id)
        .call(codelet_tools::bash::BashArgs {
            command: "printf 'once\\n' >> capacity-sentinel; printf 'authentic capacity result'"
                .into(),
            cwd: Some(directory.path().to_string_lossy().into()),
        })
        .await?;
    std::fs::create_dir(directory.path().join("messages/messages.jsonl"))?;
    let spool = PersistenceSpool::open(session.id)?;
    for _ in 0..MAX_PENDING_RECORDS {
        let id = uuid::Uuid::new_v4();
        spool.enqueue(&RecoveryRecord {
            version: 1,
            id,
            role: "user".into(),
            content: "accepted input".into(),
            metadata: serde_json::from_value(json!({"uuid":id, "type":"user", "provider":"user",
                "timestamp":chrono::Utc::now(), "message":{"role":"user",
                "content":[{"type":"text","text":"accepted input"}]}}))?,
        })?;
    }
    // @step When its authentic result settles and the output owner is dropped
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: "sentinel".into(),
        call_id: Some("native".into()),
        content: authentic.clone(),
        is_error: false,
    }));
    // @step Then further dispatch is refused with a visible scoped persistence error
    assert!(session
        .is_interrupted
        .load(std::sync::atomic::Ordering::Acquire));
    assert!(std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(event,
        StreamChunk::Error { error } if error.contains("persistence") && !error.contains(&authentic))));
    assert!(output.prepare_tool_dispatch("queued").is_err());
    let captured_pending = spool.pending_count()?;
    drop(output);
    // @step And repairing storage and explicit recovery archives the authentic result once without repeating the tool
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    let records = read(session.id)?;
    assert_eq!(
        records
            .iter()
            .filter(|record| record["message"]["content"][0]["content"] == authentic)
            .count(),
        1
    );
    assert_eq!(captured_pending, MAX_PENDING_RECORDS + 3);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("capacity-sentinel"))?,
        "once\n"
    );
    assert_eq!(JournalStore::configured(session.id)?.read()?.iter().filter(|record|
        matches!(&record.event, JournalEvent::ToolCompleted { content, .. } if content == &authentic)).count(), 1);
    Ok(())
}
