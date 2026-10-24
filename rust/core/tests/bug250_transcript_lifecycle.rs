#![doc = "Feature: spec/features/transcript-session-lifecycle.feature"]

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore, TerminalOutcome,
};
use codelet_core::persistence::{create_session, delete_session, reset_stores_for_tests};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn session_deletion_removes_its_private_transcript() -> TestResult {
    // @step Given a session with a durable terminal outcome
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let session = create_session("terminal", directory.path())?;
    let store = JournalStore::configured(session.id)?;
    store.append(&JournalRecord::new(
        JournalIdentity {
            session_id: session.id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
    ))?;
    // @step When that session is deleted
    delete_session(session.id)?;
    // @step Then its transcript records are removed with the owning session
    assert!(!store.directory().exists());
    assert!(store.read()?.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn failed_transcript_deletion_preserves_the_owning_session() -> TestResult {
    // @step Given a session whose private transcript cannot be removed
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    reset_stores_for_tests();
    let session = create_session("blocked transcript deletion", directory.path())?;
    let store = JournalStore::configured(session.id)?;
    let transcripts = directory.path().join("transcripts");
    std::fs::create_dir_all(&transcripts)?;
    std::fs::write(store.directory(), b"unavailable transcript directory")?;
    let manifest = directory
        .path()
        .join(format!("sessions/{}.json", session.id));
    let original = std::fs::read(&manifest)?;

    // @step When session deletion encounters the transcript storage failure
    let result = delete_session(session.id);

    // @step Then deletion reports the failure and retains the committed session for retry
    assert!(result.is_err(), "transcript deletion failure was ignored");
    assert_eq!(std::fs::read(&manifest)?, original);
    assert_eq!(
        std::fs::read(store.directory())?,
        b"unavailable transcript directory"
    );
    std::fs::remove_file(store.directory())?;
    delete_session(session.id)?;
    assert!(!manifest.exists());
    Ok(())
}

#[test]
fn transcript_export_retains_only_public_display_records() -> TestResult {
    // @step Given a session with a durable scrubbed provider failure
    let directory = setup_temp_directory()?;
    let session_id = Uuid::new_v4();
    let store = JournalStore::new(directory.path(), session_id);
    let record = JournalRecord::new(
        JournalIdentity {
            session_id,
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
    );
    store.append(&record)?;
    // @step When its display-only transcript is exported
    let encoded = store.export(&HashSet::new())?;
    // @step Then the exported record retains its original identity timestamp and typed public outcome
    let exported: Vec<JournalRecord> = serde_json::from_str(&encoded)?;
    assert_eq!(exported, vec![record]);
    assert!(!encoded.contains("raw_provider"));
    assert!(!encoded.contains("Bearer"));
    Ok(())
}
