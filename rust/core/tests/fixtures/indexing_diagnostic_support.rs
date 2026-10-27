//! Synthetic stores only; the root environment variable is mandatory.
pub(super) use codelet_core::persistence::messages::StoredMessage;
use codelet_core::persistence::messages::{durability, MessageStore};
use codelet_test_helpers::setup_temp_directory;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "indexing_legacy_v1.rs"]
mod legacy;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub ids: Vec<Uuid>,
}

impl Fixture {
    pub fn new(count: usize, bytes: usize) -> TestResult<Self> {
        let allowed =
            PathBuf::from(std::env::var("FSPEC_INDEX_DIAGNOSTIC_ROOT")?).canonicalize()?;
        let directory = setup_temp_directory()?;
        let path = directory.path().canonicalize()?;
        assert!(
            path.starts_with(&allowed),
            "fixture escaped the explicit diagnostic root"
        );
        codelet_common::set_data_directory(path.clone())?;
        codelet_core::persistence::reset_stores_for_tests();
        assert_eq!(codelet_common::get_data_dir()?, path);
        let mut store = MessageStore::new()?;
        let mut ids = Vec::new();
        for number in 0..count {
            ids.push(store.store("user", &format!("synthetic-{number}:{}", "x".repeat(bytes)))?);
        }
        Ok(Self { directory, ids })
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join("messages").join(name)
    }

    pub fn version(&self) -> TestResult<u32> {
        let bytes = fs::read(self.path("messages.idx"))?;
        Ok(u32::from_le_bytes(bytes[4..8].try_into()?))
    }
}

pub fn observed<T>(action: impl FnOnce() -> T) -> (T, usize) {
    let (value, operations) = durability::observe(None, action);
    (
        value,
        operations
            .iter()
            .filter(|operation| **operation == "index.write")
            .count(),
    )
}

/// Orchestration for the historical index codec, not a replacement MessageStore.
/// load/scan/save below are verbatim production functions from 2699693b^.
pub fn legacy_open(root: &Path) -> TestResult<bool> {
    let data = root.join("messages/messages.jsonl");
    let index = root.join("messages/messages.idx");
    if let Some((entries, size)) = legacy::load_index(&index) {
        if fs::metadata(&data)?.len() == size {
            if let Some((id, entry)) = entries.iter().next() {
                assert_eq!(legacy::read_message_at(&data, entry)?.id, *id);
            }
            return Ok(false);
        }
    }
    let (entries, size) = legacy::scan_jsonl_range(&data, 0)?;
    legacy::save_index(&index, &entries, size)?;
    Ok(true)
}

pub fn legacy_scan(root: &Path) -> TestResult<usize> {
    let (entries, _) = legacy::scan_jsonl_range(&root.join("messages/messages.jsonl"), 0)?;
    Ok(entries.len())
}

pub fn mixed_versions(rounds: usize) -> TestResult<(usize, usize)> {
    let fixture = Fixture::new(4, 128)?;
    let mut legacy_rebuilds = 0;
    let mut current_rebuilds = 0;
    for _ in 0..rounds {
        legacy_rebuilds += usize::from(legacy_open(fixture.directory.path())?);
        assert_eq!(fixture.version()?, 1);
        let (store, publications) = observed(MessageStore::new);
        let store = store?;
        current_rebuilds += publications;
        assert_eq!(fixture.version()?, 1);
        assert!(store.get(fixture.ids[0]).is_some());
    }
    Ok((legacy_rebuilds, current_rebuilds))
}

pub fn peer_write_publications() -> TestResult<(Vec<usize>, usize)> {
    let _fixture = Fixture::new(2, 64)?;
    let mut first = MessageStore::new()?;
    let mut second = MessageStore::new()?;
    let mut publications = Vec::new();
    for round in 0..4 {
        let writer = if round % 2 == 0 {
            &mut first
        } else {
            &mut second
        };
        let (result, count) = observed(|| writer.store("user", "fresh synthetic identity"));
        result?;
        publications.push(count);
    }
    let (result, warm) = observed(|| second.store("user", "same writer remains warm"));
    result?;
    Ok((publications, warm))
}
