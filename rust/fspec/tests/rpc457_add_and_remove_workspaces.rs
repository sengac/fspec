//! RPC-457 — add and remove workspaces through RPC and the CLI.
//!
//! Feature: spec/features/add-and-remove-workspaces.feature

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use codelet_common::workspace_registry::{self, WorkspaceRegistryError};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use common::fspec_bin;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An fspec project `<root>/<name>` (a directory holding spec/work-units.json).
fn project(root: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(path.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    Ok(std::fs::canonicalize(path)?)
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn daemon(
    start: &Path,
) -> Result<(Arc<SharedFspecService>, FspecServiceImpl), Box<dyn std::error::Error>> {
    let shared = Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    );
    Ok((Arc::clone(&shared), FspecServiceImpl::new(shared)))
}

#[test]
#[serial]
fn adding_a_directory_without_spec_is_refused() -> TestResult {
    // @step Given a directory that has no spec folder
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let plain = tempfile::tempdir()?;
    // @step When it is added as a workspace
    let added = workspace_registry::add(plain.path());
    // @step Then it is refused as not an fspec project
    assert!(
        matches!(added, Err(WorkspaceRegistryError::NotAnFspecProject(_))),
        "{added:?}"
    );
    // @step And the registry does not list it
    assert!(workspace_registry::list()?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn the_daemon_adds_and_removes_a_workspace() -> TestResult {
    // @step Given a daemon started in workspace "alpha" and a project "beta"
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha")?;
    let beta = project(root.path(), "beta")?;
    let (shared, service) = daemon(&alpha)?;
    // @step When "beta" is added over RPC and its work units are read
    let entry = service
        .clone()
        .add_workspace(tarpc::context::current(), text(&beta))
        .await?;
    assert_eq!(entry.name, "beta");
    service
        .clone()
        .list_workspace_work_units(tarpc::context::current(), text(&beta))
        .await?;
    // @step Then the workspace list includes "beta"
    let listed = service
        .clone()
        .list_workspaces(tarpc::context::current())
        .await?;
    assert!(
        listed.iter().any(|entry| entry.path == text(&beta)),
        "{listed:?}"
    );
    assert!(shared.opened_workspaces().iter().any(|w| w.path == beta));
    // @step When "beta" is removed over RPC
    service
        .clone()
        .remove_workspace(tarpc::context::current(), text(&beta))
        .await?;
    // @step Then the workspace list no longer includes "beta"
    let listed = service.list_workspaces(tarpc::context::current()).await?;
    assert!(
        !listed.iter().any(|entry| entry.path == text(&beta)),
        "{listed:?}"
    );
    // @step And the daemon no longer holds "beta" open
    assert!(!shared.opened_workspaces().iter().any(|w| w.path == beta));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn the_daemon_refuses_to_remove_its_start_workspace_or_an_unknown_one() -> TestResult {
    // @step Given a daemon started in workspace "alpha"
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha")?;
    let stranger = project(root.path(), "stranger")?;
    let (_shared, service) = daemon(&alpha)?;
    // @step When "alpha" or an unregistered directory is removed over RPC
    let start = service
        .clone()
        .remove_workspace(tarpc::context::current(), text(&alpha))
        .await;
    let unknown = service
        .remove_workspace(tarpc::context::current(), text(&stranger))
        .await;
    // @step Then each removal is refused
    assert!(start.is_err(), "{start:?}");
    assert!(unknown.is_err(), "{unknown:?}");
    Ok(())
}

fn workspaces(home: &Path, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new(fspec_bin())
        .arg("workspaces")
        .args(args)
        .env("HOME", home)
        .output()
}

#[test]
fn the_cli_adds_lists_and_removes_a_workspace() -> TestResult {
    // @step Given a project "beta"
    let home = tempfile::tempdir()?;
    let root = tempfile::tempdir()?;
    let beta = project(root.path(), "beta")?;
    // @step When "fspec workspaces add" names it
    let added = workspaces(home.path(), &["add", &text(&beta)])?;
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    // @step Then "fspec workspaces list" shows it
    let listed = workspaces(home.path(), &["list"])?;
    assert!(String::from_utf8_lossy(&listed.stdout).contains(&text(&beta)));
    // @step When "fspec workspaces remove" names it
    let removed = workspaces(home.path(), &["remove", &text(&beta)])?;
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    // @step Then "fspec workspaces list" no longer shows it
    let listed = workspaces(home.path(), &["list"])?;
    assert!(listed.status.success());
    assert!(!String::from_utf8_lossy(&listed.stdout).contains(&text(&beta)));
    Ok(())
}
