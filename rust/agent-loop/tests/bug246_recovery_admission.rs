#![doc = "Feature: spec/features/persistence-recovery-admission.feature"]

use codelet_agent_loop::{
    persist::retry_pending_persistence,
    persistence_retry::{PersistenceSpool, RecoveryRecord, MAX_PENDING_BYTES},
    BackgroundOutput,
};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_rpc_types::StreamChunk;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn unavailable_journal_retains_original_ownership() -> TestResult {
    // @step Given a real session whose archive and recovery journal are unavailable
    let (directory, session, _) = fixture()?;
    let spool = PersistenceSpool::open(session.id)?;
    let journal = spool.directory().to_path_buf();
    std::fs::remove_dir(&journal)?;
    std::fs::write(&journal, "blocked journal")?;
    let mut events = session.subscribe_to_stream();
    // @step When assistant text settles and its output owner is dropped
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Text("private authentic original α".into()));
    output.emit(StreamEvent::Done(None));
    drop(output);
    // @step Then a scoped error warns that shutdown is unsafe without exposing original content
    let errors: Vec<_> = std::iter::from_fn(|| events.try_recv().ok())
        .filter_map(|event| match event {
            StreamChunk::Error { error } => Some(error),
            _ => None,
        })
        .collect();
    assert!(errors
        .iter()
        .any(|error| error.contains("shutdown") && error.contains("unsafe")));
    assert!(errors
        .iter()
        .all(|error| !error.contains("private authentic original")));
    assert!(session.inner.lock().await.messages.is_empty());
    // @step And repairing storage and explicit recovery archives the original text once
    std::fs::remove_file(journal)?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    let envelopes = read(session.id)?;
    assert_eq!(envelopes.len(), 1);
    assert_eq!(
        envelopes[0]["message"]["content"][0]["text"],
        "private authentic original α"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn oversized_input_is_refused_before_admission() -> TestResult {
    // @step Given a real session with an empty recovery journal
    let (_directory, session, _) = fixture()?;
    let spool = PersistenceSpool::open(session.id)?;
    // @step When one envelope exceeds the recovery byte capacity
    let record = RecoveryRecord {
        version: 1,
        id: uuid::Uuid::new_v4(),
        role: "user".into(),
        content: "α".repeat(MAX_PENDING_BYTES),
        metadata: Default::default(),
    };
    let result = spool.enqueue(&record);
    // @step Then admission fails without accepting or truncating the envelope
    assert!(result.is_err());
    assert_eq!(spool.pending_count()?, 0);
    assert_eq!(record.content.len(), MAX_PENDING_BYTES * 2);
    Ok(())
}
