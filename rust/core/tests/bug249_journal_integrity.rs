#![doc = "Feature: spec/features/transcript-journal-integrity.feature"]

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore,
};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn durable_session_order_ignores_tied_and_backward_timestamps() -> TestResult {
    // @step Given acknowledged requests whose timestamps tie or move backward
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let store = JournalStore::new(directory.path(), session_id);
    let mut expected = Vec::new();
    for (request, seconds) in [(3, 20), (2, 20), (1, 10)] {
        let mut record = JournalRecord::new(
            JournalIdentity {
                session_id,
                request_id: Uuid::from_u128(request),
                attempt_id: Uuid::new_v4(),
                message_id: Uuid::new_v4(),
            },
            0,
            JournalEvent::Text {
                text: request.to_string(),
            },
        );
        record.timestamp = chrono::DateTime::from_timestamp(seconds, 0).ok_or("invalid time")?;
        store.append(&record)?;
        expected.push(record);
    }
    // @step When the transcript store is reopened
    let recovered = JournalStore::new(directory.path(), session_id).read()?;
    // @step Then records retain their exact durable admission order
    assert_eq!(recovered, expected);
    Ok(())
}

#[test]
fn missing_acknowledged_interior_deltas_fail_explicitly() -> TestResult {
    // @step Given three acknowledged deltas in one request
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let store = JournalStore::new(directory.path(), session_id);
    let identity = JournalIdentity {
        session_id,
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
    };
    let mut missing = None;
    for sequence in 0..3 {
        let record = JournalRecord::new(
            identity.clone(),
            sequence,
            JournalEvent::Text {
                text: sequence.to_string(),
            },
        );
        store.append(&record)?;
        if sequence == 1 {
            missing = Some(record);
        }
    }
    // @step When an interior delta file is missing on reopen
    let missing = serde_json::to_value(missing.ok_or("missing fixture")?)?;
    for entry in std::fs::read_dir(store.directory())? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path)?)? == missing
        {
            std::fs::remove_file(path)?;
        }
    }
    // @step Then recovery fails explicitly instead of silently shortening the answer
    assert!(JournalStore::new(directory.path(), session_id)
        .project(&HashSet::new())
        .is_err());
    Ok(())
}
