//! Feature: spec/features/shared-message-store-index-coherence.feature

use codelet_core::persistence::MessageStore;
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::{HashMap, HashSet};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// @step Given a reader opened before another store appends a message
/// @step When the reader fetches that message without writing or reopening
/// @step Then it reads the committed message with its original identity and content
#[test]
#[serial]
fn scenario_live_readers_observe_another_stores_committed_append() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let reader = MessageStore::new()?;
    let mut writer = MessageStore::new()?;

    let message_id = writer.store("assistant", "peer committed: 日本語")?;
    let message = reader.get(message_id).ok_or("peer append is missing")?;
    assert_eq!(message.id, message_id);
    assert_eq!(message.content, "peer committed: 日本語");
    Ok(())
}

/// @step Given a reader has cached a message with metadata
/// @step When another store replaces that metadata with an equally sized value
/// @step Then the reader sees the updated metadata without writing or reopening
#[test]
#[serial]
fn scenario_live_readers_invalidate_cached_metadata_after_a_peer_update() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut writer = MessageStore::new()?;
    let message_id = writer.store_with_metadata(
        "assistant",
        "shared message",
        HashMap::from([("state".into(), serde_json::json!("before"))]),
    )?;
    let reader = MessageStore::new()?;
    assert_eq!(
        reader
            .get(message_id)
            .ok_or("original is missing")?
            .metadata["state"],
        "before"
    );

    writer.update_metadata(
        message_id,
        HashMap::from([("state".into(), serde_json::json!("after!"))]),
    )?;
    let message = reader.get(message_id).ok_or("updated message is missing")?;
    assert_eq!(message.id, message_id);
    assert_eq!(message.metadata["state"], "after!");
    Ok(())
}

/// @step Given a reader has cached retained and orphaned messages and an uncached retained message
/// @step When another store removes the orphaned messages
/// @step Then the reader no longer returns the orphaned message
/// @step And it reads both retained messages with their original identities and content
#[test]
#[serial]
fn scenario_live_readers_use_current_offsets_after_another_store_compacts_history() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut writer = MessageStore::new()?;
    let orphan_id = writer.store("user", &"discarded".repeat(256))?;
    let cached_id = writer.store("assistant", "cached retained")?;
    let uncached_id = writer.store("user", "uncached retained")?;
    let reader = MessageStore::new()?;
    assert!(reader.get(orphan_id).is_some());
    assert!(reader.get(cached_id).is_some());

    assert_eq!(
        writer.cleanup_orphans(&HashSet::from([cached_id, uncached_id]))?,
        1
    );
    assert!(reader.get(orphan_id).is_none());
    for (message_id, content) in [
        (cached_id, "cached retained"),
        (uncached_id, "uncached retained"),
    ] {
        let message = reader
            .get(message_id)
            .ok_or("retained message is missing")?;
        assert_eq!(message.id, message_id);
        assert_eq!(message.content, content);
    }
    Ok(())
}
