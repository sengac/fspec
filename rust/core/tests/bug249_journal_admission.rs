#![doc = "Feature: spec/features/transcript-semantic-admission.feature"]

use codelet_core::persistence::transcript::{
    JournalError, JournalEvent, JournalIdentity, JournalRecord, JournalStore, ToolState,
};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(session_id: Uuid) -> JournalRecord {
    JournalRecord::new(
        JournalIdentity {
            session_id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Text {
            text: "acknowledged prefix".into(),
        },
    )
}

fn archived_bytes(directory: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, std::io::Error> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let bytes = std::fs::read(&path)?;
            records.push((path, bytes));
        }
    }
    records.sort();
    Ok(records)
}

#[test]
fn conflicting_record_identities_are_rejected_before_acknowledgement() -> TestResult {
    // @step Given an acknowledged transcript record with a stable identity
    for different_request in [false, true] {
        let directory = setup_temp_directory()?;
        let original = record(Uuid::new_v4());
        let store = JournalStore::new(directory.path(), original.identity.session_id);
        store.append(&original)?;
        let before = archived_bytes(store.directory())?;
        let mut conflict = original.clone();
        if different_request {
            conflict.identity.request_id = Uuid::new_v4();
        } else {
            conflict.sequence = 1;
        }

        // @step When another sequence or request reuses that record identity
        let result = store.append(&conflict);

        // @step Then admission rejects the conflict without changing the recoverable transcript
        assert!(matches!(result, Err(JournalError::Identity)));
        assert_eq!(archived_bytes(store.directory())?, before);
        assert_eq!(store.read()?, vec![original.clone()]);
        store.append(&original)?;
        assert_eq!(archived_bytes(store.directory())?, before);
    }
    Ok(())
}

fn tool_records(session_id: Uuid) -> Vec<JournalRecord> {
    let identity = record(session_id).identity;
    [
        JournalEvent::ToolRequested {
            id: "sentinel".into(),
            call_id: Some("native-sentinel".into()),
            name: "sentinel".into(),
            input: serde_json::json!({}),
        },
        JournalEvent::ToolStarted {
            id: "sentinel".into(),
        },
        JournalEvent::ToolCompleted {
            id: "sentinel".into(),
            content: "authentic captured result".into(),
            is_error: false,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(sequence, event)| JournalRecord::new(identity.clone(), sequence as u64, event))
    .collect()
}

#[test]
fn invalid_tool_lifecycle_transitions_are_rejected_before_acknowledgement() -> TestResult {
    // @step Given missing-request starts duplicate starts and cross-attempt results
    let mut failures = Vec::new();
    for (case, prefix_length, event_index, change_attempt) in [
        ("missing request", 0, 1, false),
        ("duplicate start", 2, 1, false),
        ("cross-attempt result", 2, 2, true),
    ] {
        let directory = setup_temp_directory()?;
        let session_id = Uuid::new_v4();
        let store = JournalStore::new(directory.path(), session_id);
        std::fs::create_dir_all(store.directory())?;
        let records = tool_records(session_id);
        for original in &records[..prefix_length] {
            store.append(original)?;
        }
        let before = archived_bytes(store.directory())?;
        let mut invalid = records[event_index].clone();
        invalid.id = Uuid::new_v4();
        invalid.sequence = prefix_length as u64;
        if change_attempt {
            invalid.identity.attempt_id = Uuid::new_v4();
        }
        // @step When each invalid tool transition is submitted for durable admission
        let result = store.append(&invalid);
        let unchanged = archived_bytes(store.directory())? == before;
        let recovered = store.read()?;
        let readable = store.project(&HashSet::new()).is_ok();
        if !matches!(result, Err(JournalError::Lifecycle))
            || !unchanged
            || recovered != records[..prefix_length]
            || !readable
        {
            failures.push(format!(
                "{case}: {result:?}, unchanged={unchanged}, readable={readable}"
            ));
        }
    }
    // @step Then admission rejects every invalid lifecycle without changing readable history
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn persisted_invalid_lifecycles_prevent_recovery_and_further_admission() -> TestResult {
    // @step Given persisted records containing an invalid tool lifecycle
    let mut failures = Vec::new();
    for case in ["missing request", "duplicate start", "cross-attempt result"] {
        let directory = setup_temp_directory()?;
        let session_id = Uuid::new_v4();
        let store = JournalStore::new(directory.path(), session_id);
        let records = tool_records(session_id);
        for original in &records {
            store.append(original)?;
        }
        let index = usize::from(case != "missing request") * 2;
        let mut invalid = records[index].clone();
        match case {
            "missing request" => {
                invalid.event = JournalEvent::Text {
                    text: "not a request".into(),
                }
            }
            "duplicate start" => {
                invalid.event = JournalEvent::ToolStarted {
                    id: "sentinel".into(),
                }
            }
            "cross-attempt result" => invalid.identity.attempt_id = Uuid::new_v4(),
            _ => return Err("unknown lifecycle fixture".into()),
        }
        let stored = archived_bytes(store.directory())?;
        std::fs::write(&stored[index].0, serde_json::to_vec(&invalid)?)?;
        let before = archived_bytes(store.directory())?;
        let next = JournalRecord::new(
            records[0].identity.clone(),
            3,
            JournalEvent::Text {
                text: "later delta".into(),
            },
        );
        // @step When the store is reopened and another delta is submitted
        let reopened = JournalStore::new(directory.path(), session_id);
        let recovered = reopened.read();
        let admitted = reopened.append(&next);
        let unchanged = archived_bytes(store.directory())? == before;
        if !matches!(recovered, Err(JournalError::Lifecycle))
            || !matches!(admitted, Err(JournalError::Lifecycle))
            || !unchanged
        {
            failures.push(format!(
                "{case}: read={recovered:?}, append={admitted:?}, unchanged={unchanged}"
            ));
        }
    }
    // @step Then recovery and admission report the lifecycle failure without changing evidence
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[test]
fn valid_tool_lifecycle_retries_remain_idempotent() -> TestResult {
    // @step Given an acknowledged requested started and completed tool lifecycle
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let store = JournalStore::new(directory.path(), session_id);
    let records = tool_records(session_id);
    for original in &records {
        store.append(original)?;
    }
    let before = archived_bytes(store.directory())?;
    // @step When each original record is retried after reopening the completed lifecycle
    let reopened = JournalStore::new(directory.path(), session_id);
    for _ in 0..2 {
        for original in &records {
            reopened.append(original)?;
        }
    }
    // @step Then all retries retain the exact original records and one authentic completed outcome
    assert_eq!(archived_bytes(store.directory())?, before);
    assert_eq!(reopened.read()?, records);
    let projection = reopened.project(&HashSet::new())?;
    assert_eq!(projection.tools.len(), 1);
    assert_eq!(projection.tools[0].state, ToolState::Completed);
    assert_eq!(
        projection.tools[0].result.as_deref(),
        Some("authentic captured result")
    );
    Ok(())
}

#[test]
fn corrupt_acknowledged_history_prevents_further_admission() -> TestResult {
    // @step Given an acknowledged transcript record whose stored bytes are corrupted
    for corruption in [b"{\"version\":".as_slice(), b"{}".as_slice()] {
        let directory = setup_temp_directory()?;
        let original = record(Uuid::new_v4());
        let store = JournalStore::new(directory.path(), original.identity.session_id);
        store.append(&original)?;
        let stored = archived_bytes(store.directory())?;
        std::fs::write(&stored[0].0, corruption)?;
        let before = archived_bytes(store.directory())?;
        let mut next = original.clone();
        next.id = Uuid::new_v4();
        next.sequence = 1;

        // @step When another delta is submitted for durable admission
        let result = store.append(&next);

        // @step Then admission fails without publishing another record or changing the corruption evidence
        assert!(result.is_err(), "corrupt history was acknowledged");
        assert_eq!(archived_bytes(store.directory())?, before);
        assert!(store.read().is_err());
    }
    Ok(())
}
