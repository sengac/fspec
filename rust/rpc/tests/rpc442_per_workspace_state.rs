//! Feature: spec/features/per-workspace-daemon-state.feature

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn project(root: &Path, name: &str, unit: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(
        path.join("spec/work-units.json"),
        format!(
            r#"{{"workUnits":{{"{unit}":{{"id":"{unit}","title":"{name} work","type":"story","status":"backlog"}}}}}}"#
        ),
    )?;
    Ok(std::fs::canonicalize(path)?)
}

fn service(start: &Path) -> Result<SharedFspecService, Box<dyn std::error::Error>> {
    Ok(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    )
}

fn ids(scope: &codelet_rpc::WorkspaceScope<'_>) -> Vec<String> {
    scope.work_units().into_iter().map(|unit| unit.id).collect()
}

#[tokio::test]
#[serial_test::serial]
async fn a_request_that_names_no_workspace_uses_the_start_workspace() -> TestResult {
    // @step Given a service started in workspace "alpha" with one work unit
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha", "ALPHA-001")?;
    let service = service(&alpha)?;

    // @step When a request resolves its workspace without naming one
    let scope = service.workspace(None)?;

    // @step Then it is served from "alpha" with alpha's work units and cwd
    assert_eq!(scope.cwd(), Some(alpha.as_path()));
    assert_eq!(ids(&scope), ["ALPHA-001"]);
    assert!(service.opened_workspaces().is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn naming_another_workspace_opens_it_once_with_its_own_work_units() -> TestResult {
    // @step Given a service started in workspace "alpha" and another project "beta" with a different work unit
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha", "ALPHA-001")?;
    let beta = project(root.path(), "beta", "BETA-001")?;
    let service = service(&alpha)?;

    // @step When requests name "beta" twice through different spellings of its path
    let first = service.workspace(Some(&beta))?;
    let second = service.workspace(Some(&beta.join("../beta/")))?;

    // @step Then both are served by one opened "beta" state with beta's work units and cwd
    for scope in [&first, &second] {
        assert_eq!(scope.cwd(), Some(beta.as_path()));
        assert_eq!(ids(scope), ["BETA-001"]);
    }
    let opened = service.opened_workspaces();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].name, "beta");
    assert_eq!(ids(&service.workspace(Some(&alpha))?), ["ALPHA-001"]);

    // @step And the workspace registry remembers "beta"
    assert!(codelet_common::workspace_registry::list()?
        .iter()
        .any(|workspace| workspace.path == beta));
    Ok(())
}
