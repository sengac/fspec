//! Feature: spec/features/sessions-use-their-workspace.feature

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn project(root: &std::path::Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(&path)?;
    Ok(std::fs::canonicalize(path)?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn sessions_created_without_a_workspace_use_the_start_workspace() -> TestResult {
    // @step Given a session manager whose default project is "alpha" while the process cwd is elsewhere
    let (_directory, manager, _, _) = support::manager().await?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha")?;
    assert_ne!(std::env::current_dir()?, alpha);
    manager.set_default_project(alpha.clone())?;
    manager.set_default_model("anthropic/claude-sonnet-4")?;

    // @step When a session is created without naming a workspace
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let id = handle.create_session_checked(None)?;

    // @step Then the session's project is "alpha"
    assert_eq!(
        manager.get_session(&id.value)?.project,
        alpha.to_string_lossy()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn creating_a_session_in_a_named_workspace() -> TestResult {
    // @step Given a session manager whose default project is "alpha" and another project "beta"
    let (_directory, manager, _, _) = support::manager().await?;
    let root = tempfile::tempdir()?;
    manager.set_default_project(project(root.path(), "alpha")?)?;
    manager.set_default_model("anthropic/claude-sonnet-4")?;
    let beta = project(root.path(), "beta")?;

    // @step When a session is created in workspace "beta" with the role "reviewer"
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let id = handle.create_session_in_workspace(&beta.join("../beta"), Some("reviewer".into()))?;

    // @step Then the session's project is beta's canonical path
    let session = manager.get_session(&id.value)?;
    assert_eq!(session.project, beta.to_string_lossy());

    // @step And the session has the role "reviewer"
    assert_eq!(session.get_role().as_deref(), Some("reviewer"));
    Ok(())
}

#[test]
fn without_a_default_project_there_is_no_project_to_fall_back_to() -> TestResult {
    // @step Given a session manager with no default project
    let manager = SessionManager::new();

    // @step When its default project is read
    let project = manager.default_project();

    // @step Then it is an error, not the process cwd
    assert!(
        project.is_err(),
        "BUG-437: no start workspace must not resolve to the cwd"
    );
    Ok(())
}
