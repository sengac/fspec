#![doc = "Feature: spec/features/persistence-session-ownership.feature"]

#[path = "support/bug250_retirement.rs"]
mod support;

use codelet_agent_loop::persistence_retry::PersistenceSpool;
use codelet_core::persistence::{create_session, delete_session, transcript::JournalStore};
use support::{fixture, manifest, recovery, TestResult};

#[test]
#[serial_test::serial]
fn deleted_sessions_refuse_renewed_recovery_admission() -> TestResult {
    // @step Given two live sessions with independently retained recovery originals
    let (root, session) = fixture()?;
    let other = create_session("other retained owner", root.path())?;
    let spool = PersistenceSpool::open(session)?;
    let other_spool = PersistenceSpool::open(other.id)?;
    spool.enqueue(&recovery(session))?;
    other_spool.enqueue(&recovery(other.id))?;
    // @step When one session is deleted and another original is submitted to its recovery owner
    delete_session(session)?;
    let admission = spool.enqueue(&recovery(session));
    // @step Then admission is refused without recreating its pending lane or changing the other owner
    assert!(admission.is_err());
    assert_eq!(spool.pending_count()?, 0);
    assert!(!spool.directory().exists());
    assert_eq!(other_spool.pending_count()?, 1);
    Ok(())
}

#[test]
#[serial_test::serial]
fn unreadable_manifests_preserve_exact_unacknowledged_originals() -> TestResult {
    // @step Given a recovery original retained in memory and an unreadable committed manifest
    let (root, session) = fixture()?;
    let spool = PersistenceSpool::open(session)?;
    let original = recovery(session);
    let blocked = spool.directory().join(format!("{}.stage", original.id));
    std::fs::create_dir(&blocked)?;
    assert!(spool.enqueue(&original).is_err());
    let path = manifest(root.path(), session);
    let committed = std::fs::read(&path)?;
    std::fs::write(&path, "invalid committed manifest")?;
    // @step When retry and a new recovery admission inspect that manifest
    let retried = spool.retry();
    let admitted = spool.enqueue(&recovery(session));
    // @step Then neither operation acknowledges or discards the retained original
    assert!(retried.is_err());
    assert!(admitted.is_err());
    assert_eq!(std::fs::read(&path)?, b"invalid committed manifest");
    // @step And repairing the manifest recovers only that authentic original once
    std::fs::write(&path, committed)?;
    assert_eq!(spool.pending_count()?, 1);
    std::fs::remove_dir(blocked)?;
    spool.retry()?;
    spool.retry()?;
    let records = JournalStore::configured(session)?.read()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, original.id);
    assert_eq!(spool.pending_count()?, 0);
    Ok(())
}
