//! Feature: spec/features/responsive-shared-message-storage.feature
#[path = "fixtures/indexing_diagnostic_support.rs"]
#[allow(dead_code)]
mod support;
use codelet_core::persistence::MessageStore;
use serial_test::serial;
use std::{collections::HashMap, fs};
use support::{observed, Fixture, TestResult};

#[test]
#[serial]
fn current_peers_reuse_complete_history() -> TestResult {
    // @step Given two current stores sharing verified history
    let _fixture = Fixture::new(4, 128)?;
    let mut stores = [MessageStore::new()?, MessageStore::new()?];
    // @step When they alternate fresh appends
    for round in 0..6 {
        let (result, publications) = observed(|| stores[round % 2].store("user", "fresh identity"));
        result?;
        // @step Then each append publishes its index only once
        assert_eq!(publications, 1, "round {round} rescanned before admission");
    }
    let (cold, publications) = observed(MessageStore::new);
    let mut cold = cold?;
    assert_eq!(publications, 0, "valid cold open should reuse its index");
    let (result, publications) = observed(|| cold.store("user", "cold peer admission"));
    result?;
    assert_eq!(
        publications, 1,
        "cold peer must reuse verified completeness"
    );
    Ok(())
}

#[test]
#[serial]
fn legacy_and_current_readers_coexist() -> TestResult {
    // @step Given compatible legacy and current readers
    let fixture = Fixture::new(4, 128)?;
    let original = fs::read(fixture.path("messages.jsonl"))?;
    // @step When they repeatedly reopen unchanged history
    for _ in 0..4 {
        let legacy_rebuilt = support::legacy_open(fixture.directory.path())?;
        let (store, current_publications) = observed(MessageStore::new);
        let store = store?;
        // @step Then neither reader repeatedly rebuilds the index
        assert!(
            !legacy_rebuilt,
            "current writer must publish the compatible layout"
        );
        assert_eq!(
            current_publications, 0,
            "current reader rejected compatible index"
        );
        assert!(store.get(fixture.ids[0]).is_some());
    }
    assert_eq!(fs::read(fixture.path("messages.jsonl"))?, original);
    // A deployed V2 cache remains readable without rewriting or touching history.
    let mut index = fs::read(fixture.path("messages.idx"))?;
    index[4..8].copy_from_slice(&2_u32.to_le_bytes());
    fs::write(fixture.path("messages.idx"), index)?;
    let (reader, publications) = observed(MessageStore::new);
    assert!(reader?.get(fixture.ids[0]).is_some());
    assert_eq!(publications, 0);
    Ok(())
}

#[test]
#[serial]
fn tampered_indexes_do_not_hide_originals() -> TestResult {
    // @step Given a current index with a verified generation
    let fixture = Fixture::new(3, 64)?;
    let required = fixture.ids[0];
    let mut writer = MessageStore::new()?;
    // @step When an original is omitted from that index
    let path = fixture.path("messages.idx");
    let mut bytes = fs::read(&path)?;
    let position = bytes[20..]
        .chunks_exact(28)
        .position(|entry| entry[..16] == required.as_bytes()[..])
        .ok_or("missing index entry")?;
    bytes.drain(20 + position * 28..20 + (position + 1) * 28);
    bytes[16..20].copy_from_slice(&2_u32.to_le_bytes());
    fs::write(path, bytes)?;
    // @step Then lookup recovers the intact original and conflicting append is rejected
    assert!(writer.get(required).is_some());
    let original = fs::read(fixture.path("messages.jsonl"))?;
    assert!(writer
        .store_with_id(required, "user", "conflicting identity", HashMap::new())
        .is_err());
    assert_eq!(fs::read(fixture.path("messages.jsonl"))?, original);
    Ok(())
}

#[test]
#[serial]
fn legacy_indexes_do_not_bypass_integrity() -> TestResult {
    // @step Given an index published by a legacy writer
    let fixture = Fixture::new(2, 64)?;
    support::legacy_open(fixture.directory.path())?;
    // @step When a cached record contains an invalid content hash
    let path = fixture.path("messages.jsonl");
    let mut data = fs::read(&path)?;
    let marker = b"\"content_hash\":\"";
    let hash = data
        .windows(marker.len())
        .position(|bytes| bytes == marker)
        .ok_or("missing hash")?
        + marker.len();
    data[hash] = if data[hash] == b'0' { b'1' } else { b'0' };
    fs::write(path, data)?;
    support::legacy_open(fixture.directory.path())?;
    // @step Then the current reader refuses the corrupt message
    match MessageStore::new() {
        Ok(reader) => assert!(reader.get(fixture.ids[0]).is_none()),
        Err(error) => assert!(error.contains("hash"), "{error}"),
    }
    Ok(())
}

#[test]
#[serial]
fn invalid_or_unwritable_certificates_recover_conservatively() -> TestResult {
    use codelet_core::persistence::messages::durability;
    let fixture = Fixture::new(2, 64)?;
    for invalid in [Vec::new(), b"not a certificate".to_vec()] {
        fs::write(fixture.path("messages.verified"), invalid)?;
        let mut cold = MessageStore::new()?;
        let (result, publications) = observed(|| cold.store("user", "recover unverified history"));
        result?;
        assert_eq!(publications, 2, "unverified absence must still be checked");
    }
    for boundary in [
        "verification.write",
        "verification.sync",
        "verification.rename",
    ] {
        let mut writer = MessageStore::new()?;
        let (result, _) = durability::observe(Some(boundary), || writer.store("user", boundary));
        let admitted = result?;
        let mut cold = MessageStore::new()?;
        assert!(
            cold.get(admitted).is_some(),
            "cache failure must not lose admitted data"
        );
        let (result, publications) = observed(|| cold.store("user", "conservative cold admission"));
        result?;
        assert_eq!(
            publications, 2,
            "failed cache publication must not certify absence"
        );
    }
    Ok(())
}

#[test]
#[serial]
fn replacing_history_invalidates_a_matching_length_certificate() -> TestResult {
    let fixture = Fixture::new(2, 64)?;
    let mut reader = MessageStore::new()?;
    let path = fixture.path("messages.jsonl");
    let replacement = fixture.path("replacement.jsonl");
    fs::write(&replacement, fs::read(&path)?)?;
    fs::rename(replacement, path)?;
    let (result, publications) = observed(|| reader.store("user", "after replacement"));
    result?;
    assert_eq!(
        publications, 2,
        "replacement inode must invalidate completeness"
    );
    assert!(reader.get(fixture.ids[0]).is_some());
    Ok(())
}

#[test]
fn isolated_child_writer() -> TestResult {
    let Ok(root) = std::env::var("BUG273_CHILD_STORE") else {
        return Ok(());
    };
    let root = std::path::PathBuf::from(root).canonicalize()?;
    let allowed =
        std::path::PathBuf::from(std::env::var("FSPEC_INDEX_DIAGNOSTIC_ROOT")?).canonicalize()?;
    assert!(root.starts_with(allowed));
    codelet_common::set_data_directory(root)?;
    let mut store = MessageStore::new()?;
    for _ in 0..3 {
        let (result, publications) = observed(|| store.store("user", "child process evidence"));
        result?;
        assert_eq!(publications, 1);
    }
    Ok(())
}

#[test]
#[serial]
fn concurrent_processes_share_verified_completeness() -> TestResult {
    let fixture = Fixture::new(2, 64)?;
    let executable = std::env::current_exe()?;
    let launch = || {
        std::process::Command::new(&executable)
            .args(["--exact", "isolated_child_writer", "--nocapture"])
            .env("BUG273_CHILD_STORE", fixture.directory.path())
            .spawn()
    };
    let mut first = launch()?;
    let mut second = launch()?;
    let first_status = first.wait()?;
    let second_status = second.wait()?;
    assert!(first_status.success() && second_status.success());
    let reader = MessageStore::new()?;
    assert_eq!(reader.index_len(), 8);
    assert!(reader.get(fixture.ids[0]).is_some());
    Ok(())
}

#[test]
#[serial]
fn legacy_append_invalidates_completeness_before_fresh_admission() -> TestResult {
    use std::io::Write;
    let fixture = Fixture::new(2, 64)?;
    let mut current = MessageStore::new()?;
    let id = uuid::Uuid::new_v4();
    let content = "legacy writer fixture";
    let record = support::StoredMessage {
        id,
        content_hash: codelet_core::persistence::messages::compute_hash(content.as_bytes()),
        created_at: chrono::Utc::now(),
        role: "user".into(),
        content: content.into(),
        token_count: None,
        blob_refs: vec![],
        metadata: HashMap::new(),
    };
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(fixture.path("messages.jsonl"))?;
    serde_json::to_writer(&mut file, &record)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    assert!(
        support::legacy_open(fixture.directory.path())?,
        "legacy fixture publishes its index"
    );
    let (result, publications) = observed(|| current.store("user", "fresh after legacy write"));
    result?;
    assert_eq!(
        publications, 2,
        "unverified legacy history must be checked before proving absence"
    );
    assert_eq!(
        current.get(id).ok_or("legacy original missing")?.content,
        content
    );
    let (result, publications) = observed(|| current.store("user", "verified again"));
    result?;
    assert_eq!(publications, 1);
    Ok(())
}
