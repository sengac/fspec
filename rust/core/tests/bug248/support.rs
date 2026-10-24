use codelet_core::persistence::{compute_hash, MessageStore, SessionStore, StoredMessage};
use codelet_test_helpers::setup_temp_directory;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn fixture() -> Result<(tempfile::TempDir, MessageStore, Uuid), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let _sessions = SessionStore::new()?;
    let mut store = MessageStore::new()?;
    let prefix = store.store("user", "valid prefix 日本語")?;
    Ok((directory, store, prefix))
}

pub fn record(content: &str) -> StoredMessage {
    StoredMessage {
        id: Uuid::new_v4(),
        content_hash: compute_hash(content.as_bytes()),
        created_at: chrono::Utc::now(),
        role: "assistant".into(),
        content: content.into(),
        token_count: Some(3),
        blob_refs: Vec::new(),
        metadata: Default::default(),
    }
}

pub fn append(directory: &Path, bytes: &[u8]) -> TestResult {
    std::fs::OpenOptions::new()
        .append(true)
        .open(directory.join("messages/messages.jsonl"))?
        .write_all(bytes)?;
    Ok(())
}

pub fn rebuild(directory: &Path) -> Result<MessageStore, Box<dyn std::error::Error>> {
    let index = directory.join("messages/messages.idx");
    if index.exists() {
        std::fs::remove_file(index)?;
    }
    Ok(MessageStore::new()?)
}

pub fn quarantines(directory: &Path) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(directory.join("messages"))? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "quarantine")
        {
            records.push(std::fs::read(path)?);
        }
    }
    Ok(records)
}
