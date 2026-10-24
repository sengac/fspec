#![doc = "Feature: spec/features/incremental-transcript-journal.feature"]

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore, ToolState,
};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn identity() -> JournalIdentity {
    JournalIdentity {
        session_id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
    }
}

fn append(
    store: &JournalStore,
    identity: &JournalIdentity,
    sequence: u64,
    event: JournalEvent,
) -> TestResult {
    store.append(&JournalRecord::new(identity.clone(), sequence, event))?;
    Ok(())
}

#[test]
fn requested_and_captured_tool_boundaries_remain_truthful() -> TestResult {
    // @step Given requested, dispatched and result-captured invocations
    let directory = setup_temp_directory()?;
    let identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    for (sequence, call_id) in ["requested", "started", "captured"].iter().enumerate() {
        append(
            &store,
            &identity,
            sequence as u64,
            JournalEvent::ToolRequested {
                id: call_id.to_string(),
                call_id: Some(format!("native-{call_id}")),
                name: "sentinel".into(),
                input: serde_json::json!({}),
            },
        )?;
    }
    for (sequence, call_id) in ["started", "captured"].iter().enumerate() {
        append(
            &store,
            &identity,
            sequence as u64 + 3,
            JournalEvent::ToolStarted {
                id: call_id.to_string(),
            },
        )?;
    }
    append(
        &store,
        &identity,
        5,
        JournalEvent::ToolCompleted {
            id: "captured".into(),
            content: "authentic result".into(),
            is_error: false,
        },
    )?;
    for _ in 0..2 {
        // @step When the private transcript store is reopened repeatedly
        let projection =
            JournalStore::new(directory.path(), identity.session_id).project(&HashSet::new())?;
        assert_eq!(projection.tools[0].state, ToolState::NotStarted);
        // @step Then each invocation retains its authentic not-started, outcome-unknown or completed state
        assert_eq!(projection.tools[1].state, ToolState::OutcomeUnknown);
        assert_eq!(projection.tools[2].state, ToolState::Completed);
        assert_eq!(
            projection.tools[2].result.as_deref(),
            Some("authentic result")
        );
    }
    Ok(())
}

#[test]
fn aborted_response_attempt_text_never_becomes_committed_content() -> TestResult {
    // @step Given a response attempt with visible text that is subsequently aborted
    let directory = setup_temp_directory()?;
    let mut identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    append(
        &store,
        &identity,
        0,
        JournalEvent::Text {
            text: "aborted".into(),
        },
    )?;
    append(&store, &identity, 1, JournalEvent::AttemptAborted)?;
    identity.attempt_id = Uuid::new_v4();
    // @step When another attempt succeeds and the transcript is reopened
    append(
        &store,
        &identity,
        2,
        JournalEvent::Text {
            text: "authentic".into(),
        },
    )?;
    let projection = store.project(&HashSet::new())?;
    assert_eq!(projection.records.len(), 1);
    // @step Then only the successful attempt contributes assistant text
    assert!(
        matches!(&projection.records[0].event, JournalEvent::Text { text } if text == "authentic")
    );
    Ok(())
}

#[test]
fn original_message_promotion_suppresses_duplicate_partial_content() -> TestResult {
    // @step Given a journaled assistant prefix whose immutable original message was committed
    let directory = setup_temp_directory()?;
    let identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    append(
        &store,
        &identity,
        0,
        JournalEvent::Text {
            text: "already original".into(),
        },
    )?;
    let originals = HashSet::from([identity.message_id]);
    // @step When the process stops before journal promotion acknowledgement
    // @step Then recovery suppresses the partial prefix using the original message identity
    assert!(store.project(&originals)?.records.is_empty());
    assert_eq!(store.read()?.len(), 1);
    Ok(())
}

#[test]
fn rapid_deltas_have_linear_storage_amplification() -> TestResult {
    // @step Given a stream of many fixed-size text deltas
    let directory = setup_temp_directory()?;
    let identity = identity();
    let store = JournalStore::new(directory.path(), identity.session_id);
    for sequence in 0..128 {
        // @step When every delta is durably acknowledged
        append(
            &store,
            &identity,
            sequence,
            JournalEvent::Text {
                text: "α".repeat(64),
            },
        )?;
    }
    let bytes: u64 = std::fs::read_dir(store.directory())?
        .map(|entry| {
            entry
                .and_then(|entry| entry.metadata())
                .map(|metadata| metadata.len())
        })
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum();
    assert!(
        bytes < 128 * 2048,
        "deltas must not rewrite full accumulated history"
    );
    // @step Then stored bytes grow linearly without serializing earlier history
    assert_eq!(store.project(&HashSet::new())?.records.len(), 128);
    Ok(())
}
