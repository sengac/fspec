// LOG-006 follow-on: defensive `create_dir_all` on every MessageStore
// write path, not just `save_index` + `cleanup_orphans`.
//
// Root cause:
//   MessageStore is a lazy_static process-wide singleton (see
//   rust/core/src/persistence/manifest.rs). MessageStore::new()
//   calls create_dir_all(&messages_dir) exactly once, then caches
//   `messages_dir: PathBuf` for the lifetime of the process. If the
//   directory disappears later (test teardown, sibling tooling wiping
//   ~/.fspec, codelet_common::set_data_directory swap leaving a stale
//   handle), every cached write surface returns ENOENT via different
//   error strings.
//
// The original LOG-006 patch covered `save_index` and the
// `cleanup_orphans` rewrite path. These regression tests cover the two
// remaining write paths — `store_with_metadata` and `update_metadata`
// — so that the JSONL append also self-heals after a vanished
// `messages_dir`.
//
// Without the fix these tests fail with:
//   "Failed to open messages file: No such file or directory
//    (os error 2)"
//
// All tests are `#[serial]` because they touch the process-global
// data directory configured via `codelet_common::set_data_directory`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::messages::MessageStore;
use serial_test::serial;
use std::collections::HashMap;
use tempfile::TempDir;

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    tmp
}

#[test]
#[serial]
fn store_with_metadata_recreates_missing_messages_dir() {
    // @step Given a MessageStore is constructed against a fresh temp data dir
    let data_dir = setup_data_dir();
    let messages_dir = data_dir.path().join("messages");
    let mut store = MessageStore::new().expect("MessageStore::new must succeed");

    // @step And a baseline message is appended successfully
    let _id = store
        .store_with_metadata("user", "baseline", HashMap::new())
        .expect("baseline store_with_metadata must succeed");
    assert!(messages_dir.exists(), "messages_dir must exist after store");

    // @step And the messages dir is wiped out from under the store
    //       (simulates sibling tooling, test teardown, or a
    //       set_data_directory swap leaving a stale handle).
    std::fs::remove_dir_all(&messages_dir).expect("remove messages_dir");
    assert!(
        !messages_dir.exists(),
        "precondition: messages_dir must be gone before the next store call"
    );

    // @step When store_with_metadata is invoked again on the same instance
    let result = store.store_with_metadata("user", "after-wipe", HashMap::new());

    // @step Then the call succeeds and recreates the directory
    //       (without the LOG-006 follow-on guard this would surface
    //       as `"Failed to open messages file: No such file or
    //       directory (os error 2)"`)
    assert!(
        result.is_ok(),
        "store_with_metadata must self-heal a deleted messages_dir — regressed LOG-006: {result:?}"
    );
    assert!(
        messages_dir.exists(),
        "messages_dir must be recreated by store_with_metadata"
    );
    assert!(
        messages_dir.join("messages.jsonl").exists(),
        "messages.jsonl must be written after recovery"
    );
    assert!(
        messages_dir.join("messages.idx").exists(),
        "messages.idx must be written after recovery"
    );
}

#[test]
#[serial]
fn update_metadata_recreates_missing_messages_dir() {
    // @step Given a MessageStore with one stored message
    let data_dir = setup_data_dir();
    let messages_dir = data_dir.path().join("messages");
    let mut store = MessageStore::new().expect("MessageStore::new must succeed");
    let id = store
        .store_with_metadata("assistant", "needs-metadata", HashMap::new())
        .expect("baseline store must succeed");

    // @step And the messages dir is wiped out from under the store
    std::fs::remove_dir_all(&messages_dir).expect("remove messages_dir");
    assert!(!messages_dir.exists());

    // @step When update_metadata is invoked on the cached message id
    //       (the in-memory index + LRU cache still hold the entry,
    //       so `self.get(id)` succeeds even though the on-disk JSONL
    //       was just deleted)
    let mut extra = HashMap::new();
    extra.insert(
        "stop_reason".to_string(),
        serde_json::Value::String("end_turn".to_string()),
    );
    let result = store.update_metadata(id, extra);

    // @step Then the call succeeds and recreates the directory
    //       (without the LOG-006 follow-on guard this would surface
    //       as `"Failed to open messages file: No such file or
    //       directory (os error 2)"`)
    assert!(
        result.is_ok(),
        "update_metadata must self-heal a deleted messages_dir — regressed LOG-006: {result:?}"
    );
    assert!(
        messages_dir.exists(),
        "messages_dir must be recreated by update_metadata"
    );
    assert!(
        messages_dir.join("messages.jsonl").exists(),
        "messages.jsonl must be written after recovery"
    );
}
