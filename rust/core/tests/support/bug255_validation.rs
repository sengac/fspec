use super::{identity, prefix, record, TestResult};
use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::{JournalEvent, JournalStore};
use codelet_core::persistence::{create_session, reset_stores_for_tests};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashMap;

pub fn warm_invalidation() -> TestResult {
    for change in ["tail", "missing", "writer"] {
        let directory = setup_temp_directory()?;
        let identity = identity();
        let store = JournalStore::new(directory.path(), identity.session_id);
        prefix(&store, &identity)?;
        let mut paths = std::fs::read_dir(store.directory())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        });
        paths.sort();
        let next = record(
            &identity,
            4,
            JournalEvent::Text {
                text: "other".into(),
            },
        );
        match change {
            "tail" => std::fs::write(&paths[3], b"invalid tail")?,
            "missing" => std::fs::remove_file(&paths[1])?,
            "writer" => JournalStore::new(directory.path(), identity.session_id).append(&next)?,
            _ => return Err("unknown invalidation case".into()),
        }
        if change == "writer" {
            store.append(&record(
                &identity,
                5,
                JournalEvent::Thinking {
                    thinking: "new".into(),
                },
            ))?;
            assert_eq!(store.read()?.len(), 6);
        } else {
            assert!(store.append(&next).is_err(), "warm {change} was ignored");
            assert!(store.read().is_err());
        }
    }
    Ok(())
}

pub fn recovery_scaling() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let session = create_session("streamed scaling", directory.path())?;
    let mut identity = identity();
    identity.session_id = session.id;
    let (result, operations) = observe(None, || -> TestResult {
        for sequence in 0..256 {
            let event = record(
                &identity,
                sequence,
                JournalEvent::Text {
                    text: "streamed".into(),
                },
            );
            let spool = PersistenceSpool::open(session.id)?;
            spool.enqueue(&RecoveryRecord {
                version: 1,
                id: event.id,
                role: "transcript".into(),
                content: String::new(),
                metadata: HashMap::from([("_transcript".into(), serde_json::to_value(event)?)]),
            })?;
            spool.retry()?;
        }
        Ok(())
    });
    result?;
    let reads = operations
        .iter()
        .filter(|operation| **operation == "transcript.history.read")
        .count();
    assert!(
        reads <= 512,
        "recovery streaming reread {reads} historical records"
    );
    assert_eq!(JournalStore::configured(session.id)?.read()?.len(), 256);
    let active = PersistenceSpool::open(session.id)?;
    for _ in 0..129 {
        let deleted = PersistenceSpool::open(uuid::Uuid::new_v4())?;
        assert!(deleted
            .enqueue(&RecoveryRecord {
                version: 1,
                id: uuid::Uuid::new_v4(),
                role: "user".into(),
                content: "deleted owner".into(),
                metadata: HashMap::new(),
            })
            .is_err());
    }
    let event = record(
        &identity,
        256,
        JournalEvent::Text {
            text: "after eviction".into(),
        },
    );
    let (result, operations) = observe(None, || -> TestResult {
        active.enqueue(&RecoveryRecord {
            version: 1,
            id: event.id,
            role: "transcript".into(),
            content: String::new(),
            metadata: HashMap::from([("_transcript".into(), serde_json::to_value(event)?)]),
        })?;
        active.retry()?;
        Ok(())
    });
    result?;
    assert!(
        !operations.contains(&"transcript.history.read"),
        "active writer was evicted"
    );
    assert_eq!(JournalStore::configured(session.id)?.read()?.len(), 257);
    Ok(())
}
