#![doc = "Feature: spec/features/transcript-admission-scaling.feature"]

use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore, TerminalOutcome,
};
use codelet_test_helpers::setup_temp_directory;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "support/bug255_validation.rs"]
mod validation;

fn identity() -> JournalIdentity {
    JournalIdentity {
        session_id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
    }
}

fn record(identity: &JournalIdentity, sequence: u64, event: JournalEvent) -> JournalRecord {
    JournalRecord::new(identity.clone(), sequence, event)
}

fn prefix(store: &JournalStore, identity: &JournalIdentity) -> TestResult {
    for (sequence, event) in [
        JournalEvent::ToolRequested {
            id: "authentic".into(),
            call_id: None,
            name: "read_file".into(),
            input: serde_json::json!({}),
        },
        JournalEvent::ToolStarted {
            id: "authentic".into(),
        },
        JournalEvent::ToolCompleted {
            id: "authentic".into(),
            content: "genuine result α".repeat(8192),
            is_error: false,
        },
        JournalEvent::Terminal {
            outcome: TerminalOutcome::Completed,
        },
    ]
    .into_iter()
    .enumerate()
    {
        store.append(&record(identity, sequence as u64, event))?;
    }
    Ok(())
}

mod isolation {
    use super::TestResult;
    use codelet_agent_loop::persistence_retry::{PersistenceSpool, RecoveryRecord};
    use codelet_core::persistence::{
        create_session, reset_stores_for_tests,
        transcript::{JournalEvent, JournalIdentity, JournalRecord, JournalStore},
    };
    use codelet_test_helpers::setup_temp_directory;
    use std::{collections::HashMap, fs::OpenOptions, sync::mpsc, time::Duration};
    use uuid::Uuid;

    #[test]
    #[serial_test::serial]
    fn an_unrelated_session_progresses_during_a_stalled_journal_append() -> TestResult {
        // @step Given two sessions and an exclusively locked first transcript writer
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().into())?;
        reset_stores_for_tests();
        let first = create_session("first", directory.path())?;
        let second = create_session("second", directory.path())?;
        let journal = JournalStore::configured(first.id)?;
        std::fs::create_dir_all(journal.directory())?;
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(journal.directory().join("writer.lock"))?;
        lease.lock()?;
        let spool = PersistenceSpool::open(first.id)?;
        let event = JournalRecord::new(
            JournalIdentity {
                session_id: first.id,
                request_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
            },
            0,
            JournalEvent::Text {
                text: "first".into(),
            },
        );
        spool.enqueue(&RecoveryRecord {
            version: 1,
            id: event.id,
            role: "transcript".into(),
            content: String::new(),
            metadata: HashMap::from([("_transcript".into(), serde_json::to_value(&event)?)]),
        })?;
        // @step When both sessions attempt durable transcript admission
        let (started_sender, started_receiver) = mpsc::channel();
        let first_worker = std::thread::spawn(move || {
            let _ = started_sender.send(());
            spool.retry()
        });
        started_receiver.recv()?;
        let (sender, receiver) = mpsc::channel();
        let second_worker = std::thread::spawn(move || {
            let result = PersistenceSpool::open(second.id).and_then(|spool| {
                spool.enqueue(&RecoveryRecord {
                    version: 1,
                    id: Uuid::new_v4(),
                    role: "user".into(),
                    content: "independent".into(),
                    metadata: HashMap::new(),
                })?;
                spool.retry()
            });
            let _ = sender.send(result);
        });
        let independent = receiver.recv_timeout(Duration::from_secs(3));
        drop(lease);
        first_worker.join().map_err(|_| "first worker failed")??;
        second_worker.join().map_err(|_| "second worker failed")?;
        // @step Then the second session completes without waiting for the first writer
        independent.map_err(|_| "unrelated session blocked behind global recovery lock")??;
        Ok(())
    }
}

#[test]
#[serial_test::serial]
fn long_streaming_sessions_require_bounded_admission_work() -> TestResult {
    // @step Given completed turns and a large authentic tool result in a durable session
    let directory = setup_temp_directory()?;
    let mut identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    prefix(&store, &identity)?;
    identity.request_id = Uuid::new_v4();
    let mut sequence = 0;
    // @step When growing batches of more than 128 deltas are acknowledged
    for batch in [256, 512, 1024] {
        let (result, operations) = observe(None, || -> TestResult {
            for _ in 0..batch {
                store.append(&record(
                    &identity,
                    sequence,
                    JournalEvent::Text {
                        text: "δ".repeat(16),
                    },
                ))?;
                sequence += 1;
            }
            Ok(())
        });
        result?;
        // @step Then historical record reads and lifecycle work grow linearly
        let reads = operations
            .iter()
            .filter(|operation| **operation == "transcript.history.read")
            .count();
        let admissions = operations
            .iter()
            .filter(|operation| **operation == "transcript.lifecycle.admit")
            .count();
        assert!(
            reads <= batch * 2,
            "{batch} deltas reread {reads} historical records"
        );
        assert!(
            admissions >= batch && admissions <= batch * 2,
            "{batch} deltas required {admissions} lifecycle operations"
        );
    }
    assert_eq!(store.read()?.len(), sequence as usize + 4);
    validation::recovery_scaling()?;
    Ok(())
}

#[test]
fn cold_opening_validates_immutable_history_before_admission() -> TestResult {
    validation::warm_invalidation()?;
    // @step Given a journal with acknowledged deltas and completed tools
    for invalid in [None, Some("corrupt"), Some("missing")] {
        let directory = setup_temp_directory()?;
        let identity = identity();
        let store = JournalStore::new(directory.path(), identity.session_id);
        prefix(&store, &identity)?;
        let expected = store.read()?;
        let mut paths: Vec<_> = std::fs::read_dir(store.directory())?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()?;
        paths.sort();
        match invalid {
            Some("corrupt") => std::fs::write(&paths[0], b"invalid")?,
            Some("missing") => std::fs::remove_file(&paths[1])?,
            _ => {}
        }
        // @step When a fresh writer opens intact corrupt or incomplete history
        let reopened = JournalStore::new(directory.path(), identity.session_id);
        let next = record(
            &identity,
            4,
            JournalEvent::Text {
                text: "next".into(),
            },
        );
        let result = reopened.append(&next);
        // @step Then intact history resumes exactly and invalid history refuses admission
        if invalid.is_some() {
            assert!(result.is_err());
            assert!(reopened.read().is_err());
        } else {
            result?;
            reopened.append(&next)?;
            assert_eq!(reopened.read()?, [expected, vec![next]].concat());
        }
    }
    Ok(())
}

#[test]
fn retried_publication_retains_durable_identity() -> TestResult {
    // @step Given a transcript admission whose directory acknowledgement fails
    let directory = setup_temp_directory()?;
    let identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    let first = record(
        &identity,
        0,
        JournalEvent::Text {
            text: "first".into(),
        },
    );
    let (result, _) = observe(Some("transcript.directory.sync"), || store.append(&first));
    assert!(result.is_err());
    // @step When the same record is retried and streaming continues
    store.append(&first)?;
    let next = record(
        &identity,
        1,
        JournalEvent::Text {
            text: "next".into(),
        },
    );
    store.append(&next)?;
    // @step Then recovery contains each record exactly once in durable order
    assert_eq!(
        JournalStore::new(directory.path(), identity.session_id).read()?,
        vec![first, next]
    );
    Ok(())
}
