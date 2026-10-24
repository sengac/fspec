#![doc = "Feature: spec/features/persistence-recovery-integrity.feature"]

use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use uuid::Uuid;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn unreadable_recovery_records_never_become_empty_admission_capacity() -> TestResult {
    // @step Given a retained recovery envelope whose durable record cannot be decoded
    let (directory, session, _) = fixture()?;
    let spool = PersistenceSpool::open(session.id)?;
    let id = Uuid::new_v4();
    let accepted = RecoveryRecord {
        version: 1,
        id,
        role: "user".into(),
        content: "previously accepted".into(),
        metadata: serde_json::from_value(serde_json::json!({
            "uuid":id, "type":"user", "provider":"user", "timestamp":chrono::Utc::now(),
            "message":{"role":"user", "content":[{"type":"text","text":"previously accepted"}]}
        }))?,
    };
    spool.enqueue(&accepted)?;
    let path = std::fs::read_dir(spool.directory())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .ok_or("accepted recovery file missing")?;
    let original = std::fs::read(&path)?;
    std::fs::write(&path, "not a recovery envelope")?;
    // @step When a caller attempts to admit another original envelope
    let caller_owned = RecoveryRecord {
        id: Uuid::new_v4(),
        content: "still owned by caller".into(),
        ..accepted.clone()
    };
    // @step Then admission fails explicitly and the new envelope remains with its caller
    assert!(spool.enqueue(&caller_owned).is_err());
    // @step And repairing the existing record recovers only the previously accepted envelope
    std::fs::write(&path, original)?;
    assert_eq!(spool.pending_count()?, 1);
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    spool.retry()?;
    let originals = read(session.id)?;
    assert_eq!(originals.len(), 1);
    assert_eq!(originals[0]["uuid"], accepted.id.to_string());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn corrupted_recovery_ordering_is_never_replaced_with_an_invented_sequence() -> TestResult {
    // @step Given a retained recovery envelope whose durable filename has lost its ordering coordinate
    let (_directory, session, _) = fixture()?;
    let spool = PersistenceSpool::open(session.id)?;
    let accepted = RecoveryRecord {
        version: 1,
        id: Uuid::new_v4(),
        role: "user".into(),
        content: "retained original".into(),
        metadata: Default::default(),
    };
    spool.enqueue(&accepted)?;
    let path = spool
        .directory()
        .join(format!("{:020}-{}.json", 1, accepted.id));
    let corrupted = spool
        .directory()
        .join(format!("invalid-{}.json", accepted.id));
    std::fs::rename(&path, &corrupted)?;
    let bytes = std::fs::read(&corrupted)?;
    // @step When a caller attempts to admit another original envelope
    let candidate = RecoveryRecord {
        id: Uuid::new_v4(),
        ..accepted
    };
    let admission = spool.enqueue(&candidate);
    // @step Then admission fails explicitly without accepting the new envelope or changing the retained record
    assert!(admission.is_err());
    assert_eq!(std::fs::read(&corrupted)?, bytes);
    std::fs::rename(corrupted, path)?;
    assert_eq!(spool.pending_count()?, 1);
    Ok(())
}
