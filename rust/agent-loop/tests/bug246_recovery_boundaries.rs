#![doc = "Feature: spec/features/persistence-commit-reconciliation.feature"]

use codelet_agent_loop::persist::retry_pending_persistence;
use codelet_agent_loop::persistence_retry::{
    PersistenceSpool, RecoveryRecord, MAX_PENDING_RECORDS,
};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::{get_session_messages_full, load_session};
use serde_json::json;
use uuid::Uuid;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

fn record() -> RecoveryRecord {
    let id = Uuid::new_v4();
    RecoveryRecord { version: 1, id, role: "assistant".into(), content: "owned original".into(),
        metadata: serde_json::from_value(json!({"uuid":id,"type":"assistant", "provider":"codex",
            "timestamp":chrono::Utc::now(), "message":{"role":"assistant","content":[{"type":"text","text":"owned original"}]}})).expect("valid fixture metadata") }
}

#[tokio::test]
#[serial_test::serial]
async fn message_append_survives_failed_manifest_promotion() -> TestResult {
    // @step Given a recovery envelope whose message was written but manifest promotion failed
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let pending = record();
    let spool = PersistenceSpool::open(session.id)?;
    spool.enqueue(&pending)?;
    let (failed, _) = observe(Some("manifest.rename"), || spool.retry());
    assert!(failed.is_err());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("messages/messages.jsonl"))?
            .lines()
            .count(),
        1
    );
    // @step When storage is repaired and persistence is retried twice
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    codelet_core::persistence::reset_stores_for_tests();
    // @step Then cold reopen references the same physical message exactly once
    let manifest = load_session(session.id)?;
    let records = get_session_messages_full(&manifest)?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, pending.id);
    assert_eq!(read(session.id)?[0]["uuid"], pending.id.to_string());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("messages/messages.jsonl"))?
            .lines()
            .count(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn promoted_messages_survive_failed_recovery_acknowledgement() -> TestResult {
    // @step Given a recovery envelope whose manifest was promoted but acknowledgement failed
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let pending = record();
    let spool = PersistenceSpool::open(session.id)?;
    spool.enqueue(&pending)?;
    let (failed, _) = observe(Some("manifest.directory"), || spool.retry());
    assert!(failed.is_err());
    assert_eq!(load_session(session.id)?.message_count(), 1);
    // @step When storage is repaired and persistence is retried twice
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    codelet_core::persistence::reset_stores_for_tests();
    // @step Then cold reopen references the same physical message exactly once
    let records = get_session_messages_full(&load_session(session.id)?)?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, pending.id);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("messages/messages.jsonl"))?
            .lines()
            .count(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_admission_is_bounded_and_permanent_failure_remains_visible() -> TestResult {
    // @step Given a real session whose recovery capacity is exhausted
    let (directory, session, _) = fixture()?;
    let spool = PersistenceSpool::open(session.id)?;
    for _ in 0..MAX_PENDING_RECORDS {
        spool.enqueue(&record())?;
    }
    // @step When another input would exceed recovery capacity
    let overflow = spool.enqueue(&record());
    // @step Then admission fails visibly without evicting previously retained envelopes
    assert!(overflow.is_err());
    assert!(spool.retry().is_err());
    assert_eq!(spool.pending_count()?, MAX_PENDING_RECORDS);
    // @step And cold recovery preserves accepted records without dispatching external actions
    drop(spool);
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    codelet_core::persistence::reset_stores_for_tests();
    PersistenceSpool::open(session.id)?.retry()?;
    assert_eq!(
        load_session(session.id)?.message_count(),
        MAX_PENDING_RECORDS
    );
    assert_eq!(PersistenceSpool::open(session.id)?.pending_count()?, 0);
    Ok(())
}
