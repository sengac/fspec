#![doc = "Feature: spec/features/transcript-recovery-retirement.feature"]

#[path = "support/bug250_cold.rs"]
mod storage;
#[path = "support/bug250_retirement.rs"]
mod support;

use codelet_agent_loop::persistence_retry::PersistenceSpool;
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::JournalStore;
use codelet_core::persistence::{create_session, delete_session, load_session, SessionStore};
use std::sync::mpsc;
use std::time::Duration;
use storage::{bytes, cold, pending};
use support::{fixture, manifest, recovery, TestResult};
use uuid::Uuid;

#[test]
#[serial_test::serial]
fn live_transcript_recovery_remains_authentic_and_idempotent() -> TestResult {
    // @step Given a live session with a cold durable pending transcript record
    let (root, session) = fixture()?;
    assert!(cold(root.path(), session, "enqueue")?.success());
    // @step When cold recovery retries that record twice
    let promoted = cold(root.path(), session, "promote")?;
    // @step Then exactly one authentic transcript record survives without a conversation original
    assert!(promoted.success());
    let records = JournalStore::configured(session)?.read()?;
    assert_eq!(records.len(), 1);
    assert!(
        matches!(&records[0].event, codelet_core::persistence::transcript::JournalEvent::Text { text } if text == "authentic pending prefix")
    );
    assert!(load_session(session)?.messages.is_empty());
    assert!(!root.path().join("messages/messages.jsonl").exists());
    Ok(())
}

#[test]
#[ignore = "isolated cold recovery subprocess fixture"]
fn retirement_child() -> TestResult {
    let root = std::path::PathBuf::from(std::env::var("BUG250_ROOT")?);
    let session = Uuid::parse_str(&std::env::var("BUG250_SESSION")?)?;
    codelet_common::set_data_directory(root)?;
    codelet_core::persistence::reset_stores_for_tests();
    let spool = PersistenceSpool::open(session)?;
    match std::env::var("BUG250_MODE")?.as_str() {
        "enqueue" => spool.enqueue(&recovery(session))?,
        "promote" => {
            spool.retry()?;
            spool.retry()?;
            assert_eq!(JournalStore::configured(session)?.read()?.len(), 1);
        }
        "retry" => {
            let _result = spool.retry();
            assert!(!JournalStore::configured(session)?.directory().exists());
        }
        _ => return Err("unknown cold recovery fixture mode".into()),
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn canonical_deletion_retires_durable_pending_transcript_recovery() -> TestResult {
    // @step Given a real session with a cold durable pending transcript record
    let (root, session) = fixture()?;
    assert!(cold(root.path(), session, "enqueue")?.success());
    assert_eq!(bytes(&pending(root.path(), session))?.len(), 1);
    // @step When canonical deletion settles before cold recovery runs
    delete_session(session)?;
    let retired = !pending(root.path(), session).exists();
    let replayed = cold(root.path(), session, "retry")?;
    // @step Then the pending lane is removed and recovery cannot recreate its transcript
    assert!(retired && replayed.success());
    assert!(!JournalStore::configured(session)?.directory().exists());
    Ok(())
}

#[test]
#[serial_test::serial]
fn stale_recovery_bytes_cannot_resurrect_a_deleted_transcript() -> TestResult {
    // @step Given a retained copy of a durable transcript recovery record
    let (root, session) = fixture()?;
    assert!(cold(root.path(), session, "enqueue")?.success());
    let retained = bytes(&pending(root.path(), session))?;
    delete_session(session)?;
    std::fs::create_dir_all(pending(root.path(), session))?;
    std::fs::write(&retained[0].0, &retained[0].1)?;
    // @step When those bytes are recovered after the owning session is deleted
    let replayed = cold(root.path(), session, "retry")?;
    // @step Then neither a transcript nor a conversation original is recreated
    assert!(replayed.success());
    assert!(!JournalStore::configured(session)?.directory().exists());
    assert!(load_session(session).is_err());
    assert!(!root.path().join("messages/messages.jsonl").exists());
    Ok(())
}

#[test]
#[serial_test::serial]
fn deleted_sessions_retire_caller_owned_recovery_memory() -> TestResult {
    // @step Given a transcript original retained in memory after recovery publication fails
    let (root, session) = fixture()?;
    let spool = PersistenceSpool::open(session)?;
    let original = recovery(session);
    std::fs::create_dir(spool.directory().join(format!("{}.stage", original.id)))?;
    assert!(spool.enqueue(&original).is_err());
    assert!(bytes(spool.directory())?.is_empty());
    assert_eq!(spool.pending_count()?, 1);
    // @step When the session is deleted and the retained admission is retried
    delete_session(session)?;
    let blocked = spool.directory().join(format!("{}.stage", original.id));
    if blocked.exists() {
        std::fs::remove_dir(blocked)?;
    }
    let retried = spool.retry();
    let count = spool.pending_count()?;
    // @step Then its recovery owner is empty and neither private lane is recreated
    assert!(retried.is_ok(), "{retried:?}");
    assert_eq!(count, 0);
    assert!(!pending(root.path(), session).exists());
    assert!(!JournalStore::configured(session)?.directory().exists());
    Ok(())
}

#[test]
#[serial_test::serial]
fn recovery_and_deletion_respect_active_session_ownership() -> TestResult {
    // @step Given pending transcript recovery and an active session writer transaction
    let (root, session) = fixture()?;
    PersistenceSpool::open(session)?.enqueue(&recovery(session))?;
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || -> Result<(), String> {
        SessionStore::new()?
            .update(session, |_| {
                entered_tx.send(()).map_err(|error| {
                    codelet_core::persistence::ManifestCommitError::Store(error.to_string())
                })?;
                release_rx.recv().map_err(|error| {
                    codelet_core::persistence::ManifestCommitError::Store(error.to_string())
                })?;
                Ok(())
            })
            .map(|_| ())
            .map_err(|error| error.to_string())
    });
    entered_rx.recv_timeout(Duration::from_secs(5))?;
    // @step When recovery and deletion race with that transaction
    let (retry_tx, retry_rx) = mpsc::channel();
    let retry = std::thread::spawn(move || {
        let result = PersistenceSpool::open(session).and_then(|spool| spool.retry());
        let _sent = retry_tx.send(result);
    });
    let early_retry = retry_rx.recv_timeout(Duration::from_millis(200)).ok();
    let (delete_tx, delete_rx) = mpsc::channel();
    let deletion = std::thread::spawn(move || {
        let _sent = delete_tx.send(delete_session(session));
    });
    let early_delete = delete_rx.recv_timeout(Duration::from_secs(1)).ok();
    release_tx.send(())?;
    writer.join().map_err(|_| "writer thread failed")??;
    retry.join().map_err(|_| "recovery thread failed")?;
    deletion.join().map_err(|_| "deletion thread failed")?;
    let retry_result = match early_retry.as_ref() {
        Some(result) => result.clone(),
        None => retry_rx.recv_timeout(Duration::from_secs(5))?,
    };
    delete_session(session)?;
    // @step Then recovery cannot acknowledge before ownership and deletion returns without an inverted lock wait
    assert!(early_retry.is_none(), "premature recovery: {early_retry:?}");
    assert!(matches!(early_delete, Some(Err(_))), "{early_delete:?}");
    assert!(retry_result.is_ok(), "{retry_result:?}");
    assert!(!manifest(root.path(), session).exists());
    assert!(!JournalStore::configured(session)?.directory().exists());
    Ok(())
}

#[test]
#[serial_test::serial]
fn failed_deletion_before_promotion_preserves_recovery_originals() -> TestResult {
    // @step Given durable pending transcript recovery and an unrelated live session
    let (root, session) = fixture()?;
    assert!(cold(root.path(), session, "enqueue")?.success());
    let other = create_session("unrelated", root.path())?;
    let original = std::fs::read(manifest(root.path(), session))?;
    let other_original = std::fs::read(manifest(root.path(), other.id))?;
    let retained = bytes(&pending(root.path(), session))?;
    // @step When manifest deletion fails before promotion
    let (deleted, _) = observe(Some("session.delete.manifest"), || delete_session(session));
    // @step Then both sessions and the exact pending bytes remain unchanged for retry
    assert!(deleted.is_err(), "deletion failure seam was not reached");
    assert_eq!(std::fs::read(manifest(root.path(), session))?, original);
    assert_eq!(
        std::fs::read(manifest(root.path(), other.id))?,
        other_original
    );
    assert_eq!(bytes(&pending(root.path(), session))?, retained);
    delete_session(session)?;
    assert!(!pending(root.path(), session).exists());
    assert!(load_session(other.id).is_ok());
    Ok(())
}

#[test]
#[serial_test::serial]
fn failed_cleanup_after_deletion_reports_promotion_and_remains_retryable() -> TestResult {
    // @step Given a session with durable pending private transcript data
    let (root, session) = fixture()?;
    assert!(cold(root.path(), session, "enqueue")?.success());
    let retained = bytes(&pending(root.path(), session))?;
    // @step When recovery retirement fails after manifest deletion
    let (deleted, _) = observe(Some("session.delete.recovery"), || delete_session(session));
    // @step Then deletion reports promotion and retry removes residual data without transcript resurrection
    assert!(
        deleted
            .as_ref()
            .err()
            .is_some_and(|error| error.contains("deletion promoted")),
        "{deleted:?}"
    );
    assert!(!manifest(root.path(), session).exists());
    assert_eq!(bytes(&pending(root.path(), session))?, retained);
    let event: codelet_core::persistence::transcript::JournalRecord =
        serde_json::from_value(recovery(session).metadata["_transcript"].clone())?;
    assert!(JournalStore::configured(session)?.append(&event).is_err());
    delete_session(session)?;
    assert!(!pending(root.path(), session).exists());
    assert!(!JournalStore::configured(session)?.directory().exists());
    Ok(())
}
