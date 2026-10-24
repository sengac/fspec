//! Feature: spec/features/message-index-recovery.feature

use codelet_core::persistence::MessageStore;
use codelet_test_helpers::setup_temp_directory;
use serial_test::serial;
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn alter_index(directory: &Path, required: Uuid, omit: bool) -> TestResult {
    let path = directory.join("messages/messages.idx");
    let mut bytes = std::fs::read(&path)?;
    let entries: Vec<_> = bytes[20..].chunks_exact(28).map(Vec::from).collect();
    let required_entry = entries
        .iter()
        .position(|entry| entry[..16] == required.as_bytes()[..])
        .ok_or("required index entry missing")?;
    if omit {
        let start = 20 + required_entry * 28;
        bytes.drain(start..start + 28);
        let count = u32::try_from(entries.len() - 1)?;
        bytes[16..20].copy_from_slice(&count.to_le_bytes());
    } else {
        let other = entries
            .iter()
            .find(|entry| entry[..16] != required.as_bytes()[..])
            .ok_or("second index entry missing")?;
        let start = 20 + required_entry * 28 + 16;
        bytes[start..start + 12].copy_from_slice(&other[16..]);
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

#[test]
#[serial]
fn lookup_recovers_an_original_omitted_from_an_apparently_current_index() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut writer = MessageStore::new()?;
    let required = writer.store("assistant", "recover this original")?;
    writer.store("user", "other original")?;
    let reader = MessageStore::new()?;
    alter_index(directory.path(), required, true)?;
    let recovered = reader
        .get(required)
        .ok_or("intact original not recovered")?;
    assert_eq!(recovered.id, required);
    assert_eq!(recovered.content, "recover this original");
    assert!(MessageStore::new()?.get(required).is_some());
    Ok(())
}

#[test]
#[serial]
fn lookup_repairs_an_index_entry_pointing_at_a_different_original() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut writer = MessageStore::new()?;
    let required = writer.store("assistant", "required original")?;
    writer.store("user", "unrelated original")?;
    alter_index(directory.path(), required, false)?;
    let reader = MessageStore::new()?;
    let recovered = reader
        .get(required)
        .ok_or("wrong offset was not repaired")?;
    assert_eq!(recovered.id, required);
    assert_eq!(recovered.content, "required original");
    Ok(())
}

#[test]
#[serial]
fn cleanup_preserves_referenced_originals_omitted_from_the_index() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut writer = MessageStore::new()?;
    let required = writer.store("assistant", "retained evidence")?;
    let orphan = writer.store("user", "orphan")?;
    alter_index(directory.path(), required, true)?;
    let mut cleaner = MessageStore::new()?;
    assert_eq!(cleaner.cleanup_orphans(&HashSet::from([required]))?, 1);
    let reader = MessageStore::new()?;
    let retained = reader
        .get(required)
        .ok_or("cleanup deleted required original")?;
    assert_eq!(retained.id, required);
    assert_eq!(retained.content, "retained evidence");
    assert!(reader.get(orphan).is_none());
    Ok(())
}

#[test]
#[serial]
fn cleanup_refuses_deletion_when_a_required_original_is_truly_missing() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    let mut store = MessageStore::new()?;
    let orphan = store.store("user", "preserve until references are resolved")?;
    let history = directory.path().join("messages/messages.jsonl");
    let original_bytes = std::fs::read(&history)?;
    let missing = Uuid::new_v4();
    let error = store
        .cleanup_orphans(&HashSet::from([missing]))
        .err()
        .ok_or("cleanup must reject missing required evidence")?;
    assert!(error.contains(&missing.to_string()), "{error}");
    assert_eq!(std::fs::read(history)?, original_bytes);
    assert!(MessageStore::new()?.get(orphan).is_some());
    Ok(())
}
