// Feature: spec/features/file-search-popup-uses-worktree-path-for-isolated-sessions.feature
//! BUG-541 — `search_files_in_session` searches the session's effective
//! working directory (an isolated session's worktree), resolved by the
//! session manager; an unresolvable directory is an error.
//!
//! Real temp directories and the real `FspecServiceImpl`; the stub session
//! manager is given the session's worktree as its working directory.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{TestError, TestResult};

fn write(root: &Path, file: &str) -> TestResult {
    let target = root.join(file);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, "// fixture\n")?;
    Ok(())
}

/// A daemon whose project is `project` and whose session manager is `stub`.
fn daemon(
    project: &Path,
    stub: Arc<StubSessionManagerHandle>,
) -> Result<FspecServiceImpl, TestError> {
    std::fs::create_dir_all(project.join("spec"))?;
    std::fs::write(project.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    let manager: Arc<dyn SessionManagerHandle> = stub;
    let shared = SharedFspecService::with_session_manager(
        Arc::new(WorkUnitsWatcher::new(project)?),
        manager,
    )
    .with_cwd(project.to_path_buf());
    Ok(FspecServiceImpl::new(Arc::new(shared)))
}

// Scenario: Searching files in an isolated session lists its worktree
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn searching_files_in_an_isolated_session_lists_its_worktree() -> TestResult {
    // @step Given session "s-1" works in a worktree holding "src/newfile.ts"
    let root = tempfile::tempdir()?;
    let project = root.path().join("project");
    let worktree = root.path().join("project/.fspec/worktrees/s-1");
    write(&worktree, "src/newfile.ts")?;
    let stub = Arc::new(StubSessionManagerHandle::new());
    stub.set_effective_cwd(SessionId::new("s-1"), worktree.clone());
    // @step And the project root holds "src/legacy.js", which the worktree no longer has
    write(&project, "src/legacy.js")?;
    let service = daemon(&project, stub)?;
    // @step When files starting with "src/" are searched in session "s-1" over RPC
    let found = service
        .search_files_in_session(
            tarpc::context::current(),
            SessionId::new("s-1"),
            "src/".into(),
            20,
        )
        .await?;
    // @step Then "src/newfile.ts" is found
    assert!(
        found.iter().any(|path| path.ends_with("src/newfile.ts")),
        "{found:?}"
    );
    // @step And "src/legacy.js" is not found
    assert!(
        !found.iter().any(|path| path.contains("legacy.js")),
        "{found:?}"
    );
    Ok(())
}

// Scenario: Searching files in a session whose working directory is unknown fails
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn searching_files_in_a_session_whose_working_directory_is_unknown_fails() -> TestResult {
    // @step Given the daemon cannot resolve the working directory of session "s-1"
    let root = tempfile::tempdir()?;
    let project = root.path().join("project");
    write(&project, "src/app.ts")?;
    let service = daemon(&project, Arc::new(StubSessionManagerHandle::new()))?;
    // @step When files starting with "src/" are searched in session "s-1" over RPC
    let result = service
        .search_files_in_session(
            tarpc::context::current(),
            SessionId::new("s-1"),
            "src/".into(),
            20,
        )
        .await;
    // @step Then the search fails naming session "s-1"
    match result {
        Err(error) => assert!(error.contains("s-1"), "{error}"),
        Ok(found) => {
            return Err(TestError::from(format!(
                "expected an error, found {found:?}"
            )))
        }
    }
    Ok(())
}
