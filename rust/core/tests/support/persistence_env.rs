//! Shared hermetic persistence environment for codelet-core integration
//! tests that drive the process-global MESSAGE_STORE/SESSION_STORE
//! singletons in `codelet_core::persistence`.

use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Serialise the calling test, point the data directory at a fresh temp
/// dir and reset the persistence singletons so every test starts from an
/// empty on-disk store. Keep both returned values alive for the whole test.
pub fn setup_test_env() -> (MutexGuard<'static, ()>, TempDir) {
    let guard = TEST_MUTEX
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    codelet_core::persistence::reset_stores_for_tests();
    (guard, temp_dir)
}
