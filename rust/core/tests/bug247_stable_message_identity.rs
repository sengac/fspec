#![doc = "Feature: spec/features/stable-message-commit-identities.feature"]

use codelet_core::persistence::conversation_commit::append_message_idempotent;
use codelet_core::persistence::messages::durability::observe;
use codelet_core::persistence::{ManifestCommitError, MessageStore, SessionManifest, SessionStore};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial]
fn scenario_stable_message_retries_reconcile_ambiguous_append_acknowledgement() -> TestResult {
    // @step Given a stable message identity and immutable payload
    for boundary in [
        "message.sync",
        "message.directory",
        "index.write",
        "index.sync",
        "index.rename",
        "index.directory",
    ] {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        let mut store = MessageStore::new()?;
        let id = Uuid::new_v4();
        let metadata = HashMap::from([("model".into(), serde_json::json!("stable"))]);
        // @step When message and index acknowledgement boundaries fail before repeated retries
        let (result, _) = observe(Some(boundary), || {
            store.store_with_id(id, "assistant", "original 日本語", metadata.clone())
        });
        assert!(result.is_err(), "fault not reached: {boundary}");
        let path = directory.path().join("messages/messages.jsonl");
        let original = std::fs::read(&path)?;
        // @step Then retries synchronize the existing record without appending a duplicate
        for _ in 0..2 {
            let (result, events) = observe(None, || {
                store.store_with_id(id, "assistant", "original 日本語", metadata.clone())
            });
            assert_eq!(result?, id);
            assert!(events.contains(&"message.sync"), "{boundary}: {events:?}");
            assert!(
                events.contains(&"message.directory"),
                "{boundary}: {events:?}"
            );
            assert_eq!(std::fs::read(&path)?, original);
        }
        // @step And a cold reader retains its identity payload and original timestamp
        std::fs::remove_file(directory.path().join("messages/messages.idx"))?;
        let reopened = MessageStore::new()?;
        assert_eq!(reopened.index_len(), 1);
        let stored = reopened.get(id).ok_or("missing stable record")?;
        let first: codelet_core::persistence::StoredMessage = serde_json::from_slice(&original)?;
        assert_eq!(stored.id, id);
        assert_eq!(stored.created_at, first.created_at);
        assert_eq!(stored.content, "original 日本語");
        assert_eq!(stored.metadata, metadata);
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_stable_message_identities_reject_conflicting_payloads() -> TestResult {
    // @step Given an acknowledged message with a stable identity
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    codelet_core::persistence::reset_stores_for_tests();
    let mut store = MessageStore::new()?;
    let id = Uuid::new_v4();
    let metadata = HashMap::from([("model".into(), serde_json::json!("stable"))]);
    store.store_with_id(id, "assistant", "original", metadata.clone())?;
    let session = SessionStore::new()?.create("identity conflict", directory.path())?;
    append_message_idempotent(session.id, id, "assistant", "original", metadata.clone())?;
    let manifest_path = directory
        .path()
        .join(format!("sessions/{}.json", session.id));
    let manifest = std::fs::read(&manifest_path)?;
    let path = directory.path().join("messages/messages.jsonl");
    let original = std::fs::read(&path)?;
    // @step When its role content or metadata conflicts on retry
    for (role, content, retry_metadata) in [
        ("user", "original", metadata.clone()),
        ("assistant", "changed", metadata),
        ("assistant", "original", HashMap::new()),
    ] {
        let result = store.store_with_id(id, role, content, retry_metadata.clone());
        // @step Then the conflicting retry fails without altering the archived bytes
        assert!(result.is_err(), "conflicting identity accepted");
        assert!(append_message_idempotent(session.id, id, role, content, retry_metadata).is_err());
        assert_eq!(std::fs::read(&path)?, original);
        assert_eq!(std::fs::read(&manifest_path)?, manifest);
    }
    Ok(())
}

#[test]
#[serial]
fn scenario_concurrent_stable_message_retries_append_exactly_once() -> TestResult {
    // @step Given independent writers with the same stable identity and payload
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let id = Uuid::new_v4();
    let barrier = Arc::new(Barrier::new(2));
    // @step When both writers acknowledge the message concurrently
    let writers: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || -> Result<_, String> {
                let mut store = MessageStore::new()?;
                barrier.wait();
                store.store_with_id(id, "user", "same immutable payload", HashMap::new())
            })
        })
        .collect();
    for writer in writers {
        assert_eq!(writer.join().map_err(|_| "writer failed")??, id);
    }
    // @step Then rebuilding the derived index retrieves exactly one physical record
    let content = std::fs::read_to_string(directory.path().join("messages/messages.jsonl"))?;
    assert_eq!(content.lines().count(), 1);
    std::fs::remove_file(directory.path().join("messages/messages.idx"))?;
    let reopened = MessageStore::new()?;
    assert_eq!(reopened.index_len(), 1);
    assert_eq!(
        reopened.get(id).ok_or("missing message")?.content,
        "same immutable payload"
    );
    Ok(())
}

#[test]
#[serial]
fn scenario_conversation_retry_reconciles_failed_manifest_acknowledgement() -> TestResult {
    // @step Given a stable conversation message and committed empty session
    for boundary in ["manifest.sync", "manifest.rename", "manifest.directory"] {
        let directory = setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let session = SessionStore::new()?.create("stable conversation", directory.path())?;
        let id = Uuid::new_v4();
        let append =
            || append_message_idempotent(session.id, id, "user", "stable original", HashMap::new());
        // @step When manifest promotion or acknowledgement fails before repeated retries
        let (result, events) = observe(Some(boundary), append);
        assert!(
            events.contains(&"message.directory"),
            "{boundary}: {result:?}"
        );
        assert!(result.is_err(), "fault not reached: {boundary}");
        if boundary == "manifest.directory" {
            assert!(matches!(result, Err(ManifestCommitError::Promoted { .. })));
        }
        let path = directory.path().join("messages/messages.jsonl");
        let original = std::fs::read(&path)?;
        assert_eq!(append()?, id);
        assert_eq!(append()?, id);
        assert_eq!(std::fs::read(&path)?, original);
        // @step Then the manifest references the original physical message exactly once
        let manifest: SessionManifest = serde_json::from_slice(&std::fs::read(
            directory
                .path()
                .join(format!("sessions/{}.json", session.id)),
        )?)?;
        assert_eq!(manifest.messages.len(), 1);
        assert_eq!(manifest.messages[0].message_id, id);
        std::fs::remove_file(directory.path().join("messages/messages.idx"))?;
        assert_eq!(
            MessageStore::new()?
                .get(id)
                .ok_or("missing original")?
                .content,
            "stable original"
        );
    }
    Ok(())
}
