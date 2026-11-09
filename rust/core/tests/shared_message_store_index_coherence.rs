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
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let first = MessageStore::new()?;
    let second = MessageStore::new()?;

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
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let first = MessageStore::new()?;
    let shared_id = first.store("assistant", "shared message")?;
    let second = MessageStore::new()?;
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
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let first = MessageStore::new()?;
    let orphan_id = first.store("user", &"orphan".repeat(128))?;
    let retained_id = first.store("assistant", "retained message")?;
    let second = MessageStore::new()?;
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
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let first = MessageStore::new()?;
    let second = MessageStore::new()?;

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
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let stores = (0..4)
        .map(|_| MessageStore::new())
        .collect::<Result<Vec<_>, _>>()?;
    let barrier = Arc::new(Barrier::new(stores.len()));
    let writers: Vec<_> = stores
        .into_iter()
        .enumerate()
        .map(|(writer, store)| {
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

/// BUG-519: give the message history and its saved index one shared
/// modification time, an hour in the past, as when both files are written
/// within one filesystem timestamp tick. A rebuilt index gets a new time.
fn share_modification_time(data_dir: &Path) -> TestResult {
    let shared = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for name in ["messages/messages.jsonl", "messages/messages.idx"] {
        std::fs::OpenOptions::new()
            .write(true)
            .open(data_dir.join(name))?
            .set_modified(shared)?;
    }
    Ok(())
}

/// @step Given a store has committed messages and saved its index
/// @step And the saved index has the same modification time as the message history
/// @step When a cold store reopens the history
/// @step Then it reads every committed message with its original content
/// @step And reopening uses the saved index without rebuilding it
#[test]
#[serial]
fn scenario_reopening_reuses_an_index_published_in_the_same_timestamp_tick() -> TestResult {
    let data_dir = setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let store = MessageStore::new()?;
    let mut messages = Vec::new();
    for sequence in 0..8 {
        let content = format!("same-tick message {sequence}");
        messages.push((store.store("user", &content)?, content));
    }
    drop(store);

    share_modification_time(data_dir.path())?;

    let expected: Vec<_> = messages
        .iter()
        .map(|(message_id, content)| (*message_id, content.as_str()))
        .collect();
    assert_saved_messages(data_dir.path(), &expected)
}

/// @step Given a store has committed two messages and saved its index
/// @step And the message history is rewritten to the same size with the two records swapped
/// @step And the saved index has the same modification time as the message history
/// @step When a cold store reopens the history
/// @step Then it rebuilds the index from the history
/// @step And it reads every committed message with its original content
#[test]
#[serial]
fn scenario_reopening_rebuilds_a_same_tick_index_whose_last_record_no_longer_matches() -> TestResult
{
    let data_dir = setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let store = MessageStore::new()?;
    let first_id = store.store("user", "first record before the rewrite")?;
    let second_id = store.store("assistant", "second record before the rewrite")?;
    drop(store);

    let history_path = data_dir.path().join("messages/messages.jsonl");
    let history = std::fs::read_to_string(&history_path)?;
    let lines: Vec<&str> = history.lines().collect();
    assert_eq!(lines.len(), 2);
    std::fs::write(&history_path, format!("{}\n{}\n", lines[1], lines[0]))?;
    share_modification_time(data_dir.path())?;
    let index_path = data_dir.path().join("messages/messages.idx");
    let stale_index = std::fs::read(&index_path)?;

    let reopened = MessageStore::new()?;
    assert_ne!(
        std::fs::read(&index_path)?,
        stale_index,
        "an index whose last record no longer matches the history must be rebuilt"
    );
    for (message_id, content) in [
        (first_id, "first record before the rewrite"),
        (second_id, "second record before the rewrite"),
    ] {
        let message = reopened
            .get(message_id)
            .ok_or_else(|| format!("committed message {message_id} is missing"))?;
        assert_eq!(message.content, content);
    }
    Ok(())
}

/// BUG-519: rewrite the completeness proof (messages.verified) for the
/// current generation of both files, as the store does when it publishes the
/// proof after an index written in the same timestamp tick as its history.
#[cfg(unix)]
fn prove_current_generation(data_dir: &Path) -> TestResult {
    use std::os::unix::fs::MetadataExt;
    let stamp = |name: &str| -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let metadata = std::fs::metadata(data_dir.join("messages").join(name))?;
        let time =
            |time: std::time::SystemTime| -> Result<serde_json::Value, Box<dyn std::error::Error>> {
                let since = time.duration_since(std::time::UNIX_EPOCH)?;
                Ok(serde_json::json!({
                    "secs_since_epoch": since.as_secs(),
                    "nanos_since_epoch": since.subsec_nanos(),
                }))
            };
        Ok(serde_json::json!({
            "length": metadata.len(),
            "modified": time(metadata.modified()?)?,
            "created": metadata.created().ok().map(time).transpose()?,
            "identity": [metadata.dev(), metadata.ino(), metadata.ctime(), metadata.ctime_nsec()],
        }))
    };
    let proof_path = data_dir.join("messages/messages.verified");
    let mut proof: serde_json::Value = serde_json::from_slice(&std::fs::read(&proof_path)?)?;
    assert_eq!(proof["version"], 1, "unexpected completeness proof version");
    proof["generation"] = serde_json::json!({
        "messages": stamp("messages.jsonl")?,
        "index": stamp("messages.idx")?,
    });
    std::fs::write(proof_path, serde_json::to_vec(&proof)?)?;
    Ok(())
}

fn index_publications<T>(action: impl FnOnce() -> T) -> (T, usize) {
    let (value, operations) =
        codelet_core::persistence::messages::durability::observe(None, action);
    let publications = operations
        .iter()
        .filter(|operation| **operation == "index.write")
        .count();
    (value, publications)
}

/// @step Given a store has committed messages and saved its index
/// @step And the saved index has the same modification time as the message history
/// @step And its completeness proof matches the current generation of both files
/// @step When a cold store reopens the history and appends a fresh message
/// @step Then reopening uses the saved index without rebuilding it
/// @step And the append publishes the index once without rescanning the history
#[cfg(unix)]
#[test]
#[serial]
fn scenario_reopening_keeps_the_completeness_of_a_same_tick_index_whose_proof_matches() -> TestResult
{
    let data_dir = setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let store = MessageStore::new()?;
    let first_id = store.store("user", "certified same-tick message")?;
    drop(store);

    share_modification_time(data_dir.path())?;
    prove_current_generation(data_dir.path())?;

    let (reopened, publications) = index_publications(MessageStore::new);
    let reopened = reopened?;
    assert_eq!(
        publications, 0,
        "a proven same-tick index must not be rebuilt"
    );
    let (appended, publications) =
        index_publications(|| reopened.store("user", "fresh message after reopening"));
    let appended = appended?;
    assert_eq!(
        publications, 1,
        "a proven same-tick index must not be rescanned before an append"
    );
    for (message_id, content) in [
        (first_id, "certified same-tick message"),
        (appended, "fresh message after reopening"),
    ] {
        let message = reopened
            .get(message_id)
            .ok_or_else(|| format!("committed message {message_id} is missing"))?;
        assert_eq!(message.content, content);
    }
    Ok(())
}
