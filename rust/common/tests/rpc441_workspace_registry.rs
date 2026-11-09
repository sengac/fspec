//! Feature: spec/features/workspace-registry.feature

use codelet_common::workspace_registry::{self, WorkspaceRegistryError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn data_dir() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    Ok(data)
}

#[test]
#[serial_test::serial]
fn different_spellings_of_a_path_open_one_workspace() -> TestResult {
    // @step Given a project directory named "proj" reachable through a relative path, a trailing slash and a symlink
    let _data = data_dir()?;
    let root = tempfile::tempdir()?;
    let project = root.path().join("proj");
    std::fs::create_dir(&project)?;
    let link = root.path().join("link-to-proj");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&project, &link)?;
    let spellings = [
        root.path().join("proj/../proj"),
        root.path().join("proj/"),
        #[cfg(unix)]
        link,
    ];

    // @step When each spelling is opened through the workspace registry
    let opened = spellings
        .iter()
        .map(|path| workspace_registry::open(path))
        .collect::<Result<Vec<_>, _>>()?;

    // @step Then every open returns the same canonical path and the display name "proj"
    let canonical = std::fs::canonicalize(&project)?;
    for workspace in &opened {
        assert_eq!(workspace.path, canonical);
        assert_eq!(workspace.name, "proj");
    }

    // @step And the registry lists exactly one workspace
    assert_eq!(workspace_registry::list()?, vec![opened[0].clone()]);
    Ok(())
}

#[test]
#[serial_test::serial]
fn opened_workspaces_are_remembered_across_restarts() -> TestResult {
    // @step Given two project directories have been opened through the workspace registry
    let data = data_dir()?;
    let root = tempfile::tempdir()?;
    let (first, second) = (root.path().join("zeta"), root.path().join("alpha"));
    std::fs::create_dir(&first)?;
    std::fs::create_dir(&second)?;
    let opened = vec![
        workspace_registry::open(&first)?,
        workspace_registry::open(&second)?,
    ];

    // @step When the registry is read again from the same data directory
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let listed = workspace_registry::list()?;

    // @step Then both workspaces are listed in the order they were first opened
    assert_eq!(listed, opened);
    assert_eq!(
        listed.iter().map(|w| w.name.as_str()).collect::<Vec<_>>(),
        ["zeta", "alpha"]
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn a_path_that_is_not_a_directory_is_rejected() -> TestResult {
    // @step Given a path to a regular file and a path that does not exist
    let _data = data_dir()?;
    let root = tempfile::tempdir()?;
    let file = root.path().join("notes.txt");
    std::fs::write(&file, "not a project")?;
    let missing = root.path().join("missing");

    // @step When each is opened through the workspace registry
    // @step Then each open fails with a not-a-directory error
    for path in [&file, &missing] {
        assert!(matches!(
            workspace_registry::open(path),
            Err(WorkspaceRegistryError::NotADirectory(_))
        ));
    }

    // @step And nothing is recorded in the registry
    assert!(workspace_registry::list()?.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn workspaces_whose_directory_was_removed_are_not_listed() -> TestResult {
    // @step Given two opened workspaces
    let _data = data_dir()?;
    let root = tempfile::tempdir()?;
    let (kept, removed) = (root.path().join("kept"), root.path().join("removed"));
    std::fs::create_dir(&kept)?;
    std::fs::create_dir(&removed)?;
    let kept = workspace_registry::open(&kept)?;
    workspace_registry::open(&removed)?;

    // @step When one workspace directory is deleted
    std::fs::remove_dir(&removed)?;

    // @step Then the registry lists only the remaining workspace
    assert_eq!(workspace_registry::list()?, vec![kept]);
    Ok(())
}
