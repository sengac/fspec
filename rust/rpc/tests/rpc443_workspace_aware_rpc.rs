//! Feature: spec/features/workspace-aware-rpc.feature

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
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

fn rpc(start: &Path) -> Result<FspecServiceImpl, Box<dyn std::error::Error>> {
    Ok(FspecServiceImpl::new(Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    )))
}

fn ctx() -> tarpc::context::Context {
    tarpc::context::current()
}

#[tokio::test]
#[serial_test::serial]
async fn listing_workspaces_puts_the_start_workspace_first() -> TestResult {
    // @step Given a daemon service started in "alpha" that has also opened "beta"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha", "ALPHA-001")?;
    let beta = project(root.path(), "beta", "BETA-001")?;
    let service = rpc(&alpha)?;
    service
        .clone()
        .list_workspace_work_units(ctx(), beta.to_string_lossy().into_owned())
        .await?;

    // @step When a client lists workspaces
    let workspaces = service.list_workspaces(ctx()).await?;

    // @step Then the list is alpha marked as the start workspace followed by beta
    let summary: Vec<_> = workspaces
        .iter()
        .map(|entry| (entry.name.as_str(), entry.path.as_str(), entry.start))
        .collect();
    assert_eq!(
        summary,
        [
            ("alpha", alpha.to_string_lossy().as_ref(), true),
            ("beta", beta.to_string_lossy().as_ref(), false),
        ]
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn reading_a_named_workspaces_work_units() -> TestResult {
    // @step Given a daemon service started in "alpha" and a project "beta" with work unit "BETA-001"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha", "ALPHA-001")?;
    let beta = project(root.path(), "beta", "BETA-001")?;
    let service = rpc(&alpha)?;

    // @step When a client lists the work units of "beta"
    let units = service
        .clone()
        .list_workspace_work_units(ctx(), beta.to_string_lossy().into_owned())
        .await?;

    // @step Then the result contains only "BETA-001"
    assert_eq!(
        units.into_iter().map(|unit| unit.id).collect::<Vec<_>>(),
        ["BETA-001"]
    );

    // @step And listing work units without a workspace still returns alpha's work units
    let start = service.list_work_units(ctx()).await;
    assert_eq!(
        start.into_iter().map(|unit| unit.id).collect::<Vec<_>>(),
        ["ALPHA-001"]
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn naming_a_path_that_is_not_a_directory_is_rejected() -> TestResult {
    // @step Given a daemon service started in "alpha"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = project(root.path(), "alpha", "ALPHA-001")?;
    let service = rpc(&alpha)?;

    // @step When a client lists the work units of a path that does not exist
    let missing = root.path().join("missing").to_string_lossy().into_owned();
    let result = service
        .list_workspace_work_units(ctx(), missing.clone())
        .await;

    // @step Then the call returns an error naming that path
    let Err(error) = result else {
        return Err("a missing workspace must be rejected".into());
    };
    assert!(error.contains(&missing), "{error}");
    Ok(())
}
