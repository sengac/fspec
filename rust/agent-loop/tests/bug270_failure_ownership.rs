#![cfg(unix)]
#![doc = "Feature: spec/features/streaming-admission-failure-ownership.feature"]

use codelet_agent_loop::persist::retry_pending_persistence;
use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::{JournalEvent, JournalRecord, JournalStore};
use codelet_rpc_types::StreamChunk;

#[path = "support/streaming_speed.rs"]
mod fixture;
use fixture::TestResult;

#[tokio::test]
#[serial_test::serial]
async fn directory_and_root_barrier_failures_retain_the_provisional_journal_identity() -> TestResult
{
    for boundary in ["transcript.directory.sync", "transcript.root.sync"] {
        // @step Given a stream with a failing journal directory or root durability barrier
        let (directory, session, output) = fixture::background()?;
        let spool = PersistenceSpool::open(session.id)?;
        let mut published = session.subscribe_to_stream();
        // @step When a delta reaches the provisional journal but cannot be acknowledged
        observe(Some(boundary), || {
            output.emit(StreamEvent::Text("provisional original α".into()));
        });
        // @step Then publication is withheld and recovery owns that same journal record
        assert!(!std::iter::from_fn(|| published.try_recv().ok())
            .any(|chunk| matches!(chunk, StreamChunk::Text { .. })));
        assert_eq!(spool.pending_count()?, 1);
        let provisional = JournalStore::new(directory.path(), session.id).read()?;
        assert_eq!(provisional.len(), 1);
        let pending_path = std::fs::read_dir(spool.directory())?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| entry.path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .ok_or("missing durable recovery record")?;
        let pending: RecoveryRecord = serde_json::from_slice(&std::fs::read(pending_path)?)?;
        let original: JournalRecord =
            serde_json::from_value(pending.metadata["_transcript"].clone())?;
        assert_eq!(pending.id, original.id);
        assert_eq!(provisional, vec![original.clone()]);
        // @step When persistence is retried twice after repair
        drop(output);
        retry_pending_persistence(&session.id)?;
        retry_pending_persistence(&session.id)?;
        // @step Then only the original journal identity is recovered
        assert_eq!(
            JournalStore::new(directory.path(), session.id).read()?,
            vec![original]
        );
        assert_eq!(spool.pending_count()?, 0);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[serial_test::serial]
async fn combined_journal_and_recovery_staging_failure_retains_caller_ownership() -> TestResult {
    // @step Given a stream whose journal record barrier and recovery staging both fail
    let (directory, session, output) = fixture::background()?;
    let spool = PersistenceSpool::open(session.id)?;
    let status = std::process::Command::new("/bin/chmod")
        .args(["+a", "everyone deny add_file"])
        .arg(spool.directory())
        .status()?;
    assert!(status.success(), "fixture must deny recovery staging");
    let mut published = session.subscribe_to_stream();
    // @step When a delta is emitted and its output owner is dropped
    let (_, operations) = observe(Some("transcript.record.sync"), || {
        output.emit(StreamEvent::Text("memory retained original α".into()));
    });
    drop(output);
    // @step Then no text is published and the original remains in bounded recovery ownership
    let pending_count = spool.pending_count();
    let staged_count = std::fs::read_dir(spool.directory())?.count();
    let status = std::process::Command::new("/bin/chmod")
        .args(["-a", "everyone deny add_file"])
        .arg(spool.directory())
        .status()?;
    assert!(status.success(), "fixture must repair recovery staging");
    assert_eq!(pending_count?, 1);
    assert_eq!(staged_count, 0, "recovery staging must actually fail");
    assert!(!operations.contains(&"transcript.record.sync"));
    assert!(!std::iter::from_fn(|| published.try_recv().ok())
        .any(|chunk| matches!(chunk, StreamChunk::Text { .. })));
    assert!(JournalStore::new(directory.path(), session.id)
        .read()?
        .is_empty());
    drop(spool);
    // @step When storage is repaired and persistence is retried twice
    let reopened = PersistenceSpool::open(session.id)?;
    assert_eq!(reopened.pending_count()?, 1);
    retry_pending_persistence(&session.id)?;
    let original = JournalStore::new(directory.path(), session.id).read()?;
    retry_pending_persistence(&session.id)?;
    // @step Then cold recovery contains the authentic delta exactly once
    assert_eq!(original.len(), 1);
    assert!(
        matches!(&original[0].event, JournalEvent::Text { text } if text == "memory retained original α")
    );
    assert_eq!(
        JournalStore::new(directory.path(), session.id).read()?,
        original
    );
    assert_eq!(reopened.pending_count()?, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_warmed_writer_cannot_resurrect_a_deleted_session() -> TestResult {
    // @step Given a stream that durably published a delta before its session was deleted
    let (directory, session, output) = fixture::background()?;
    let mut published = session.subscribe_to_stream();
    output.emit(StreamEvent::Text("before deletion".into()));
    assert!(
        matches!(published.try_recv()?, StreamChunk::Text { text, .. } if text == "before deletion")
    );
    assert_eq!(
        JournalStore::new(directory.path(), session.id)
            .read()?
            .len(),
        1
    );
    codelet_core::persistence::delete_session(session.id)?;
    // @step When that same stream emits another delta
    output.emit(StreamEvent::Text("after deletion".into()));
    // @step Then the later delta is not published and the deleted journal stays absent
    assert!(!std::iter::from_fn(|| published.try_recv().ok())
        .any(|chunk| matches!(chunk, StreamChunk::Text { .. })));
    assert!(!JournalStore::new(directory.path(), session.id)
        .directory()
        .exists());
    assert_eq!(PersistenceSpool::open(session.id)?.pending_count()?, 0);
    Ok(())
}
