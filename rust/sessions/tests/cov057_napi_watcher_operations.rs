//! Feature: spec/features/napi-bindings-for-watcher-operations.feature
//!
//! COV-057: the retired NAPI `session_get_parent` / `session_get_watchers`
//! bindings are served today by the `SessionManagerHandle` surface of the
//! real `codelet_sessions::SessionManager` (the handle every transport
//! dispatches to): `get_subordinate(watcher)` answers "parent of a watcher"
//! and `get_supervisors(parent)` answers "watchers of a parent". Sessions
//! are created through `SessionManagerHandle::create_session` as in
//! git021_checkpoint_requires_isolation.rs.
//!
//! multi-thread runtime is REQUIRED: `create_session` bridges sync->async via
//! `tokio::task::block_in_place`.

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A real manager with a hermetic data directory; the returned temp dir
/// must outlive the test.
fn manager() -> Result<(tempfile::TempDir, Arc<SessionManager>), Box<dyn std::error::Error>> {
    let data_dir = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    offline_catalog::seed_offline_catalog()?;
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    Ok((data_dir, manager))
}

fn session(handle: &dyn SessionManagerHandle) -> Result<SessionId, String> {
    handle.create_session_checked(None)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_get_parent_of_a_watcher_session() -> TestResult {
    let (_data_dir, manager) = manager()?;
    let handle: &dyn SessionManagerHandle = &*manager;
    // @step Given a watcher session "watcher-uuid" watching parent "parent-uuid"
    let parent = session(handle)?;
    let watcher = session(handle)?;
    handle.add_supervisor(&parent, &watcher)?;
    // @step When I call session_get_parent with "watcher-uuid"
    let result = handle.get_subordinate(&watcher);
    // @step Then it should return "parent-uuid"
    assert_eq!(result, Some(parent));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_get_parent_of_a_regular_session_returns_none() -> TestResult {
    let (_data_dir, manager) = manager()?;
    let handle: &dyn SessionManagerHandle = &*manager;
    // @step Given a regular session "regular-uuid" with no parent
    let regular = session(handle)?;
    manager.get_session(&regular.value)?;
    // @step When I call session_get_parent with "regular-uuid"
    let result = handle.get_subordinate(&regular);
    // @step Then it should return None
    assert_eq!(result, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_get_watchers_of_a_parent_session() -> TestResult {
    let (_data_dir, manager) = manager()?;
    let handle: &dyn SessionManagerHandle = &*manager;
    // @step Given a parent session "parent-uuid"
    let parent = session(handle)?;
    // @step And watcher session "watcher-1-uuid" watching "parent-uuid"
    let watcher_1 = session(handle)?;
    handle.add_supervisor(&parent, &watcher_1)?;
    // @step And watcher session "watcher-2-uuid" watching "parent-uuid"
    let watcher_2 = session(handle)?;
    handle.add_supervisor(&parent, &watcher_2)?;
    // @step When I call session_get_watchers with "parent-uuid"
    let watchers = handle.get_supervisors(&parent)?;
    // @step Then it should return ["watcher-1-uuid", "watcher-2-uuid"]
    assert_eq!(watchers, vec![watcher_1, watcher_2]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_get_watchers_of_a_session_with_no_watchers() -> TestResult {
    let (_data_dir, manager) = manager()?;
    let handle: &dyn SessionManagerHandle = &*manager;
    // @step Given a session "lonely-uuid" with no watchers
    let lonely = session(handle)?;
    let unrelated_parent = session(handle)?;
    let unrelated_watcher = session(handle)?;
    handle.add_supervisor(&unrelated_parent, &unrelated_watcher)?;
    // @step When I call session_get_watchers with "lonely-uuid"
    let watchers = handle.get_supervisors(&lonely)?;
    // @step Then it should return an empty array
    assert!(
        watchers.is_empty(),
        "lonely session has watchers {watchers:?}"
    );
    assert_eq!(
        handle.get_supervisors(&unrelated_parent)?,
        vec![unrelated_watcher]
    );
    Ok(())
}
