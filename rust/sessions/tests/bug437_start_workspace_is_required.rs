//! BUG-437 — the start workspace is required and validated.
//!
//! Feature: spec/features/start-workspace-is-required.feature

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_manager_without_a_start_workspace_has_no_default_project() {
    // @step Given a session manager whose start workspace was never set
    let manager = SessionManager::new();
    // @step When its default project is requested
    let project = manager.default_project();
    // @step Then the request fails naming the missing start workspace
    let error = project.err().unwrap_or_default();
    assert!(error.contains("start workspace"), "{error}");
}

#[test]
fn a_start_workspace_that_cannot_be_canonicalized_is_refused() -> TestResult {
    // @step Given a session manager
    let manager = SessionManager::new();
    let missing = codelet_test_helpers::setup_temp_directory()?
        .path()
        .join("does-not-exist");
    // @step When the start workspace is set to a path that does not exist
    let result = manager.set_default_project(missing);
    // @step Then setting it fails with the canonicalize error
    let error = result.err().unwrap_or_default();
    assert!(error.contains("does-not-exist"), "{error}");
    assert!(manager.default_project().is_err());
    Ok(())
}

#[test]
fn a_second_different_start_workspace_is_refused() -> TestResult {
    // @step Given a session manager whose start workspace is set
    let manager = SessionManager::new();
    let first = codelet_test_helpers::setup_temp_directory()?;
    let second = codelet_test_helpers::setup_temp_directory()?;
    manager.set_default_project(first.path().to_path_buf())?;
    // @step When a different start workspace is set
    let result = manager.set_default_project(second.path().to_path_buf());
    // @step Then setting it fails naming both workspaces and the first stays in effect
    let error = result.err().unwrap_or_default();
    let first_canonical = std::fs::canonicalize(first.path())?;
    assert!(
        error.contains(&first_canonical.display().to_string()),
        "{error}"
    );
    assert_eq!(manager.default_project()?, first_canonical);
    Ok(())
}

#[test]
fn the_effective_cwd_of_an_unknown_session_is_an_error() -> TestResult {
    // @step Given a session manager with a start workspace
    let manager = SessionManager::new();
    let workspace = codelet_test_helpers::setup_temp_directory()?;
    manager.set_default_project(workspace.path().to_path_buf())?;
    // @step When the effective cwd of a session it does not know is requested
    let cwd = SessionManagerHandle::get_effective_cwd(
        manager.as_ref(),
        &SessionId::new(uuid::Uuid::new_v4().to_string()),
    );
    // @step Then the request fails instead of returning the start workspace
    assert!(cwd.is_err(), "{cwd:?}");
    Ok(())
}
