//! BUG-404 — tool-layer isolation fails closed on failed session lookups.
//!
//! Feature: spec/features/tool-isolation-fails-closed.feature

// The shared WT-004 fixtures; the WT-004 suite that owns them allows these
// lints for the same helpers.
#[path = "wt004_session_tool_callbacks_support/mod.rs"]
#[allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod support;

use codelet_tools::facade::validate_and_resolve_path;
use support::{
    create_plain, fresh_git_repo, manager_with_seeded_cache, register_tool_callbacks, ENV_GUARD,
};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_file_operation_for_an_unknown_session_is_refused() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    // @step Given a session manager registered for the tool callbacks
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);
    // @step When a file path is validated for a session id the manager does not know
    let unknown = Uuid::new_v4();
    let outside = tempfile::tempdir()?;
    let path = outside.path().join("loose.txt");
    let result = validate_and_resolve_path(unknown, path.to_str().ok_or("utf8")?, "read");
    // @step Then the operation is refused with an error naming the session
    let error = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(error.contains(&unknown.to_string()), "{error}");
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_known_non_isolated_session_is_not_restricted() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    // @step Given a session manager registered for the tool callbacks with a non-isolated session
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);
    let repo = fresh_git_repo();
    let session_id = create_plain(&manager, repo.path().to_str().ok_or("utf8")?).await;
    // @step When a file path outside any worktree is validated for that session
    let outside = tempfile::tempdir()?;
    let path = outside.path().join("loose.txt");
    let resolved = validate_and_resolve_path(
        Uuid::parse_str(&session_id)?,
        path.to_str().ok_or("utf8")?,
        "read",
    )?;
    // @step Then the path is allowed
    assert_eq!(resolved, path);
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_standalone_auxiliary_agent_is_not_restricted() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    // @step Given a session manager registered for the tool callbacks
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);
    // @step And an auxiliary agent registered with no owning session
    let project = fresh_git_repo();
    let auxiliary = Uuid::new_v4();
    let _scope =
        codelet_tools::tool_admission::register_auxiliary(auxiliary, project.path().to_path_buf())?;
    // @step When a file path is validated for that auxiliary agent
    let path = project.path().join("notes.txt");
    let resolved = validate_and_resolve_path(auxiliary, path.to_str().ok_or("utf8")?, "read")?;
    // @step Then the path is allowed
    assert_eq!(resolved, path);
    Ok(())
}
