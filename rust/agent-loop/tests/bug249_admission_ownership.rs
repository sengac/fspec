#![doc = "Feature: spec/features/transcript-caller-admission.feature"]

use codelet_agent_loop::{
    persistence_retry::{PersistenceSpool, RecoveryRecord, MAX_PENDING_RECORDS},
    BackgroundOutput,
};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn full_recovery_admission_retains_the_callers_original_delta() -> TestResult {
    // @step Given a full bounded recovery owner and an unacknowledged assistant delta
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    std::fs::write(directory.path().join("transcripts"), "unavailable")?;
    let spool = PersistenceSpool::open(session.id)?;
    for _ in 0..MAX_PENDING_RECORDS {
        let record = JournalRecord::new(
            JournalIdentity {
                session_id: session.id,
                request_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
            },
            0,
            JournalEvent::AttemptStarted,
        );
        spool.enqueue(&RecoveryRecord {
            version: 1,
            id: record.id,
            role: "transcript".into(),
            content: String::new(),
            metadata: HashMap::from([("_transcript".into(), json!(record))]),
        })?;
    }
    let mut chunks = session.subscribe_to_stream();
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Text("caller-owned prefix".into()));
    // @step And persistence interruption is not labelled user cancellation
    assert_eq!(
        session.interruption_reason(),
        None,
        "storage failure is not a user stop"
    );
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(
            chunk,
            codelet_rpc_types::StreamChunk::Text { .. }
        ));
    }
    // @step When storage is repaired and the same output completes recovery
    std::fs::remove_file(directory.path().join("transcripts"))?;
    codelet_agent_loop::persist::retry_pending_persistence(&session.id)?;
    output.emit(StreamEvent::Done(None));
    let records = JournalStore::configured(session.id)?.read()?;
    // @step Then the original delta is archived once without being published or lost
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(&record.event,
        JournalEvent::Text { text } if text == "caller-owned prefix"))
            .count(),
        1
    );
    // @step And the recovered conversation contains the authentic prefix exactly once
    let originals = support::read(session.id)?;
    assert_eq!(originals.len(), 1);
    assert_eq!(
        originals[0]["message"]["content"][0]["text"],
        "caller-owned prefix"
    );
    Ok(())
}
