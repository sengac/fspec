//! Feature: spec/features/shared-message-store-index-coherence.feature

use codelet_core::persistence::messages::MessageStore;
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_saved_messages(data_dir: &Path, messages: &[(Uuid, &str)]) -> TestResult {
    let index_path = data_dir.join("messages/messages.idx");
    let saved_index = std::fs::read(&index_path)?;
    let modified = std::fs::metadata(&index_path)?.modified()?;
    let reopened = MessageStore::new()?;
    assert_eq!(reopened.index_len(), messages.len());
    for (message_id, content) in messages {
        let message = reopened
            .get(*message_id)
            .ok_or_else(|| format!("committed message {message_id} is missing"))?;
        assert_eq!(message.id, *message_id);
        assert_eq!(message.content, *content);
    }
    assert_eq!(std::fs::read(&index_path)?, saved_index);
    assert_eq!(std::fs::metadata(index_path)?.modified()?, modified);
    Ok(())
}

/// @step Given two independent stores opened before either writes a message
/// @step When the stores take turns appending distinct messages
/// @step Then a cold reopened store reads every committed message with its original content
/// @step And reopening uses the saved index without rebuilding it
#[test]
#[serial]
fn scenario_independent_stores_retain_every_committed_append_after_reopening() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let mut first = MessageStore::new()?;
    let mut second = MessageStore::new()?;

    let first_id = first.store("user", "first committed message")?;
    let second_id = second.store("assistant", "second committed message, different length")?;
    let third_id = first.store("user", "third message: 日本語")?;
    drop(first);
    drop(second);

    assert_saved_messages(
        data_dir.path(),
        &[
            (first_id, "first committed message"),
            (second_id, "second committed message, different length"),
            (third_id, "third message: 日本語"),
        ],
    )
}

/// @step Given two independent stores with a cached copy of the same message
/// @step When one store appends a message and updates the shared message metadata
/// @step And the other store adds different metadata to the shared message
/// @step Then a cold reopened store reads both messages and both metadata updates
#[test]
#[serial]
fn scenario_metadata_updates_preserve_another_stores_messages_and_metadata() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let mut first = MessageStore::new()?;
    let shared_id = first.store("assistant", "shared message")?;
    let mut second = MessageStore::new()?;
    assert!(second.get(shared_id).is_some());

    let other_id = first.store("user", "peer append")?;
    first.update_metadata(
        shared_id,
        HashMap::from([("first".into(), serde_json::json!("preserve me"))]),
    )?;
    second.update_metadata(
        shared_id,
        HashMap::from([("second".into(), serde_json::json!("also preserve me"))]),
    )?;
    drop(first);
    drop(second);

    assert_saved_messages(
        data_dir.path(),
        &[(shared_id, "shared message"), (other_id, "peer append")],
    )?;
    let reopened = MessageStore::new()?;
    let shared = reopened.get(shared_id).ok_or("shared message missing")?;
    assert_eq!(shared.metadata["first"], "preserve me");
    assert_eq!(shared.metadata["second"], "also preserve me");
    Ok(())
}

/// @step Given two independent stores opened with retained and orphaned messages
/// @step When one store removes the orphaned messages
/// @step And the other store appends a new message
/// @step Then a cold reopened store reads the retained and new messages correctly
/// @step And the orphaned messages stay absent from the index
#[test]
#[serial]
fn scenario_appending_after_another_store_compacts_history_uses_current_offsets() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let mut first = MessageStore::new()?;
    let orphan_id = first.store("user", &"orphan".repeat(128))?;
    let retained_id = first.store("assistant", "retained message")?;
    let mut second = MessageStore::new()?;
    assert!(second.get(orphan_id).is_some());
    assert!(second.get(retained_id).is_some());

    assert_eq!(first.cleanup_orphans(&HashSet::from([retained_id]))?, 1);
    let new_id = second.store("user", "new message")?;
    assert!(second.get(orphan_id).is_none());
    let retained = second.get(retained_id).ok_or("retained message missing")?;
    assert_eq!(retained.content, "retained message");
    drop(first);
    drop(second);

    assert_saved_messages(
        data_dir.path(),
        &[(retained_id, "retained message"), (new_id, "new message")],
    )?;
    assert!(MessageStore::new()?.get(orphan_id).is_none());
    Ok(())
}

/// @step Given two independent stores opened before either writes a message
/// @step When one store appends a referenced message and an orphaned message
/// @step And the other store cleans up history retaining the referenced message
/// @step Then cleanup removes exactly the orphaned message
/// @step And a cold reopened store reads the referenced message
#[test]
#[serial]
fn scenario_cleanup_retains_referenced_messages_appended_by_another_store() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let mut first = MessageStore::new()?;
    let mut second = MessageStore::new()?;

    let retained_id = first.store("user", "referenced peer message")?;
    let orphan_id = first.store("assistant", "orphaned peer message")?;
    assert_eq!(second.cleanup_orphans(&HashSet::from([retained_id]))?, 1);
    drop(first);
    drop(second);

    assert_saved_messages(data_dir.path(), &[(retained_id, "referenced peer message")])?;
    assert!(MessageStore::new()?.get(orphan_id).is_none());
    Ok(())
}

/// @step Given independent stores opened before any writer appends
/// @step When the stores append distinct messages concurrently
/// @step Then a cold reopened store reads every committed message with its original content
/// @step And reopening uses the saved index without rebuilding it
#[test]
#[serial]
fn scenario_concurrent_independent_writers_retain_every_message() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let stores = (0..4)
        .map(|_| MessageStore::new())
        .collect::<Result<Vec<_>, _>>()?;
    let barrier = Arc::new(Barrier::new(stores.len()));
    let writers: Vec<_> = stores
        .into_iter()
        .enumerate()
        .map(|(writer, mut store)| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || -> Result<Vec<(Uuid, String)>, String> {
                barrier.wait();
                let mut messages = Vec::new();
                for sequence in 0..8 {
                    let content = format!(
                        "writer {writer}, message {sequence}: {}",
                        "x".repeat(writer)
                    );
                    messages.push((store.store("user", &content)?, content));
                }
                Ok(messages)
            })
        })
        .collect();
    let results: Vec<_> = writers
        .into_iter()
        .map(std::thread::JoinHandle::join)
        .collect();
    let mut messages = Vec::new();
    for result in results {
        messages.extend(result.map_err(|_| "message writer thread failed")??);
    }
    let expected: Vec<_> = messages
        .iter()
        .map(|(message_id, content)| (*message_id, content.as_str()))
        .collect();
    assert_saved_messages(data_dir.path(), &expected)
}
