//! Feature: spec/features/concurrent-message-index-persistence.feature

use codelet_core::persistence::messages::{MessageStore, StoredMessage};
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Barrier};
use uuid::Uuid;

#[test]
#[serial]
fn scenario_concurrent_session_stores_rebuild_a_shared_message_index(
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = setup_temp_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let messages_dir = data_dir.path().join("messages");
    std::fs::create_dir_all(&messages_dir)?;
    let mut data =
        std::io::BufWriter::new(std::fs::File::create(messages_dir.join("messages.jsonl"))?);
    let mut message_ids = Vec::new();
    for message_number in 0..1024 {
        let message = StoredMessage {
            id: Uuid::new_v4(),
            content_hash: String::new(),
            created_at: chrono::Utc::now(),
            role: "user".into(),
            content: format!("message {message_number}"),
            token_count: Some(3),
            blob_refs: Vec::new(),
            metadata: HashMap::new(),
        };
        serde_json::to_writer(&mut data, &message)?;
        data.write_all(b"\n")?;
        message_ids.push(message.id);
    }
    data.flush()?;
    let message_ids = Arc::new(message_ids);
    let index_path = messages_dir.join("messages.idx");

    for _round in 0..3 {
        if index_path.exists() {
            std::fs::remove_file(&index_path)?;
        }
        let barrier = Arc::new(Barrier::new(16));
        let mut readers = Vec::new();
        for _reader in 0..16 {
            let barrier = barrier.clone();
            let message_ids = message_ids.clone();
            readers.push(std::thread::spawn(move || -> Result<(), String> {
                barrier.wait();
                let store = MessageStore::new()?;
                for (message_number, message_id) in message_ids.iter().enumerate() {
                    let message = store
                        .get(*message_id)
                        .ok_or_else(|| format!("stored message {message_id} is missing"))?;
                    assert_eq!(message.content, format!("message {message_number}"));
                }
                Ok(())
            }));
        }
        let results: Vec<_> = readers
            .into_iter()
            .map(std::thread::JoinHandle::join)
            .collect();
        for result in results {
            result.map_err(|_| "index reader thread failed")??;
        }

        let modified = std::fs::metadata(&index_path)?.modified()?;
        let reopened = MessageStore::new()?;
        for message_id in message_ids.iter() {
            assert!(reopened.get(*message_id).is_some());
        }
        assert_eq!(std::fs::metadata(&index_path)?.modified()?, modified);
        assert_eq!(std::fs::read_dir(&messages_dir)?.count(), 2);
    }
    Ok(())
}
