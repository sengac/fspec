#![cfg(unix)]
#![doc = "Feature: spec/features/streaming-durable-admission-fast-path.feature"]

use codelet_agent_loop::persist::retry_pending_persistence;
use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::{JournalEvent, JournalRecord, JournalStore};
use codelet_rpc_types::StreamChunk;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

#[path = "support/streaming_speed.rs"]
mod fixture;
use fixture::TestResult;

#[path = "support/streaming_records.rs"]
mod journal_fixture;

#[tokio::test]
#[serial_test::serial]
async fn healthy_stream_admission_does_not_require_a_writable_recovery_directory() -> TestResult {
    // @step Given a live session with an empty recovery lane that cannot hold record files
    let (directory, session, output) = fixture::background()?;
    let spool = PersistenceSpool::open(session.id)?;
    std::fs::set_permissions(spool.directory(), std::fs::Permissions::from_mode(0o500))?;
    let mut published = session.subscribe_to_stream();
    // @step When text and thinking deltas are emitted
    let (_, operations) = observe(None, || {
        output.emit(StreamEvent::Text("durable text".into()));
        output.emit(StreamEvent::Thinking("durable thinking".into()));
    });
    std::fs::set_permissions(spool.directory(), std::fs::Permissions::from_mode(0o700))?;
    // @step Then both deltas are published after their journal durability barriers
    assert!(
        matches!(published.try_recv()?, StreamChunk::Text { text, .. } if text == "durable text")
    );
    assert!(
        matches!(published.try_recv()?, StreamChunk::Thinking { thinking, .. } if thinking == "durable thinking")
    );
    for boundary in [
        "transcript.record.sync",
        "transcript.directory.sync",
        "transcript.root.sync",
    ] {
        assert_eq!(
            operations
                .iter()
                .filter(|operation| **operation == boundary)
                .count(),
            2
        );
    }
    // @step And cold recovery contains both deltas exactly once without recovery files
    assert_eq!(std::fs::read_dir(spool.directory())?.count(), 0);
    assert_eq!(spool.pending_count()?, 0);
    let records = JournalStore::new(directory.path(), session.id).read()?;
    assert_eq!(records.len(), 2);
    assert!(matches!(&records[0].event, JournalEvent::Text { text } if text == "durable text"));
    assert!(
        matches!(&records[1].event, JournalEvent::Thinking { thinking } if thinking == "durable thinking")
    );
    assert_ne!(records[0].id, records[1].id);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn failed_direct_admission_retains_exact_identity_for_recovery() -> TestResult {
    // @step Given a live session whose journal directory durability barrier fails
    let (directory, session, output) = fixture::background()?;
    let spool = PersistenceSpool::open(session.id)?;
    let mut published = session.subscribe_to_stream();
    // @step When a text delta is emitted
    observe(Some("transcript.directory.sync"), || {
        output.emit(StreamEvent::Text("retained original α".into()));
    });
    // @step Then the delta is not published and its exact record is retained for recovery
    assert!(!std::iter::from_fn(|| published.try_recv().ok())
        .any(|chunk| matches!(chunk, StreamChunk::Text { .. })));
    assert_eq!(spool.pending_count()?, 1);
    let pending_path = std::fs::read_dir(spool.directory())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .ok_or("missing durable recovery record")?;
    let pending: RecoveryRecord = serde_json::from_slice(&std::fs::read(pending_path)?)?;
    let original: JournalRecord = serde_json::from_value(pending.metadata["_transcript"].clone())?;
    assert_eq!(pending.id, original.id);
    assert!(
        matches!(&original.event, JournalEvent::Text { text } if text == "retained original α")
    );
    // @step When storage recovers and persistence is retried twice
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    // @step Then cold recovery contains the original identity exactly once
    assert_eq!(
        JournalStore::new(directory.path(), session.id).read()?,
        vec![original]
    );
    assert_eq!(spool.pending_count()?, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn existing_recovery_work_is_not_bypassed_by_direct_admission() -> TestResult {
    // @step Given an older pending transcript event and a later stream delta
    let (directory, session, output) = fixture::background()?;
    let spool = PersistenceSpool::open(session.id)?;
    let mut older = journal_fixture::records(1).remove(0);
    older.identity.session_id = session.id;
    spool.enqueue(&RecoveryRecord {
        version: 1,
        id: older.id,
        role: "transcript".into(),
        content: String::new(),
        metadata: HashMap::from([("_transcript".into(), serde_json::to_value(&older)?)]),
    })?;
    let mut published = session.subscribe_to_stream();
    // @step When the later delta is emitted
    output.emit(StreamEvent::Text("later delta".into()));
    // @step Then cold recovery contains the older event before the later delta
    let records = JournalStore::new(directory.path(), session.id).read()?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], older);
    assert!(matches!(&records[1].event, JournalEvent::Text { text } if text == "later delta"));
    // @step And the later delta is published only after recovery drains
    assert_eq!(spool.pending_count()?, 0);
    assert!(
        matches!(published.try_recv()?, StreamChunk::Text { text, .. } if text == "later delta")
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn deleted_sessions_cannot_publish_through_direct_admission() -> TestResult {
    // @step Given a stream whose persisted session has been deleted
    let (directory, session, output) = fixture::background()?;
    codelet_core::persistence::delete_session(session.id)?;
    let mut published = session.subscribe_to_stream();
    // @step When text and thinking deltas are emitted
    output.emit(StreamEvent::Text("deleted text".into()));
    output.emit(StreamEvent::Thinking("deleted thinking".into()));
    // @step Then neither delta is published and no transcript is recreated
    assert!(
        !std::iter::from_fn(|| published.try_recv().ok()).any(|chunk| matches!(
            chunk,
            StreamChunk::Text { .. } | StreamChunk::Thinking { .. }
        ))
    );
    assert!(!JournalStore::new(directory.path(), session.id)
        .directory()
        .exists());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn tool_dispatch_still_requires_durable_journal_admission() -> TestResult {
    // @step Given a tool invocation published through healthy stream admission
    let (directory, session, output) = fixture::background()?;
    let mut published = session.subscribe_to_stream();
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "durable-tool".into(),
        call_id: None,
        name: "fixture".into(),
        args: serde_json::json!({}),
    }));
    assert!(matches!(
        published.try_recv()?,
        StreamChunk::ToolCall { .. }
    ));
    // @step When its dispatch journal barrier fails
    let (dispatch, _) = observe(Some("transcript.directory.sync"), || {
        output.prepare_tool_dispatch("durable-tool")
    });
    // @step Then tool dispatch is refused
    assert!(dispatch.is_err());
    // @step When storage recovers and persistence is retried twice
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    // @step Then the invocation and dispatch identities each occur exactly once
    let records = JournalStore::new(directory.path(), session.id).read()?;
    assert_eq!(records.len(), 2);
    assert!(
        matches!(&records[0].event, JournalEvent::ToolRequested { id, .. } if id == "durable-tool")
    );
    assert!(matches!(&records[1].event, JournalEvent::ToolStarted { id } if id == "durable-tool"));
    assert_ne!(records[0].id, records[1].id);
    assert_eq!(PersistenceSpool::open(session.id)?.pending_count()?, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn healthy_admission_leaves_recovery_lane_permissions_unchanged() -> TestResult {
    // @step Given a live session with an empty read-only recovery lane
    let (_directory, session, output) = fixture::background()?;
    let spool = PersistenceSpool::open(session.id)?;
    std::fs::set_permissions(spool.directory(), std::fs::Permissions::from_mode(0o500))?;
    let mut published = session.subscribe_to_stream();
    // @step When a healthy text delta is durably published
    output.emit(StreamEvent::Text("healthy durable delta".into()));
    let mode = std::fs::metadata(spool.directory())?.permissions().mode() & 0o777;
    let entries = std::fs::read_dir(spool.directory())?.count();
    std::fs::set_permissions(spool.directory(), std::fs::Permissions::from_mode(0o700))?;
    assert!(matches!(published.try_recv()?, StreamChunk::Text { .. }));
    // @step Then the recovery lane remains read-only and empty
    assert_eq!(
        mode, 0o500,
        "healthy admission must not mutate its recovery lane"
    );
    assert_eq!(entries, 0);
    Ok(())
}
