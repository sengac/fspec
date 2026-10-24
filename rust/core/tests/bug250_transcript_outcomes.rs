#![doc = "Feature: spec/features/durable-transcript-outcomes.feature"]

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, JournalStore, TerminalOutcome,
};
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashSet;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn record(outcome: TerminalOutcome) -> JournalRecord {
    JournalRecord::new(
        JournalIdentity {
            session_id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            message_id: Uuid::new_v4(),
        },
        0,
        JournalEvent::Terminal { outcome },
    )
}

#[test]
fn partial_text_and_one_original_provider_error_survive_reopen() -> TestResult {
    // @step Given partial assistant text followed by a provider failure
    let directory = setup_temp_directory()?;
    let mut terminal = record(TerminalOutcome::ProviderError);
    terminal.sequence = 1;
    let store = JournalStore::new(directory.path(), terminal.identity.session_id);
    let text = JournalRecord::new(
        terminal.identity.clone(),
        0,
        JournalEvent::Text {
            text: "partial answer".into(),
        },
    );
    store.append(&text)?;
    store.append(&terminal)?;
    // @step When the terminal event is durably archived
    store.append(&terminal)?;
    for _ in 0..2 {
        // @step When terminal persistence is retried and the archive is reopened twice
        let records = store.project(&HashSet::new())?.records;
        assert_eq!(records, vec![text.clone(), terminal.clone()]);
        // @step Then the text precedes exactly one error with its original timestamp
        assert_eq!(records[1].timestamp, terminal.timestamp);
    }
    Ok(())
}

#[test]
fn cancellation_and_loop_interruption_remain_distinct() -> TestResult {
    // @step Given a genuine user cancellation and a genuine loop interruption
    let directory = setup_temp_directory()?;
    for outcome in [
        TerminalOutcome::UserCancelled,
        TerminalOutcome::LoopInterrupted,
    ] {
        let terminal = record(outcome);
        let store = JournalStore::new(directory.path(), terminal.identity.session_id);
        store.append(&terminal)?;
        // @step When their terminal records are reopened
        assert_eq!(store.read()?, vec![terminal]);
    }
    assert_ne!(
        TerminalOutcome::UserCancelled.message(),
        TerminalOutcome::LoopInterrupted.message()
    );
    // @step Then the archive retains each original reason without inventing a tool cancellation
    Ok(())
}

#[test]
fn failed_compaction_never_becomes_successful_completion() -> TestResult {
    // @step Given a failed compaction before DAG publication
    let directory = setup_temp_directory()?;
    let terminal = record(TerminalOutcome::CompactionFailed);
    let store = JournalStore::new(directory.path(), terminal.identity.session_id);
    store.append(&terminal)?;
    let records = store.project(&HashSet::new())?.records;
    // @step When its terminal record is reopened
    // @step Then the transcript contains a compaction failure and no successful settlement
    assert_eq!(records, vec![terminal]);
    assert_eq!(
        TerminalOutcome::CompactionFailed.message(),
        "Compaction failed before publication."
    );
    Ok(())
}

#[test]
fn terminal_diagnostics_cannot_persist_sensitive_provider_payloads() -> TestResult {
    // @step Given a provider error containing credentials, a prompt and a tool payload
    let directory = setup_temp_directory()?;
    let terminal = record(TerminalOutcome::ProviderError);
    let store = JournalStore::new(directory.path(), terminal.identity.session_id);
    // @step When the terminal event is durably archived
    store.append(&terminal)?;
    let encoded = serde_json::to_string(&store.read()?)?;
    // @step Then only the typed scrubbed public notice is stored
    for sensitive in [
        "sk-secret",
        "Bearer",
        "prompt",
        "tool_payload",
        "raw_provider",
    ] {
        assert!(!encoded.contains(sensitive));
    }
    let mut invalid = serde_json::to_value(&terminal)?;
    invalid["event"]["raw_provider"] = serde_json::json!("Bearer sk-secret prompt tool_payload");
    assert!(serde_json::from_value::<JournalRecord>(invalid).is_err());
    Ok(())
}
