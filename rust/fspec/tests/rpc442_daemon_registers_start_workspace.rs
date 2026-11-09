//! Feature: spec/features/daemon-start-workspace-registration.feature

mod common;

use std::io::{BufRead, BufReader};
use std::process::Stdio;

#[test]
fn the_daemon_records_its_start_workspace_in_the_registry() -> Result<(), Box<dyn std::error::Error>>
{
    // @step Given a project directory used as the daemon's start workspace
    let (workspace_dir, _) = common::make_workspace(&[("AUTH-001", "Login", "backlog")]);
    let home = tempfile::tempdir()?;
    let runtime = tempfile::tempdir()?;

    // @step When the daemon service is built for it
    let mut child = common::IsolatedCommand::new(common::fspec_bin())
        .arg("--workspace")
        .arg(workspace_dir.path())
        .arg("daemon")
        .env("HOME", home.path())
        .env("FSPEC_USER_DIR", home.path().join(".fspec"))
        .env("XDG_RUNTIME_DIR", runtime.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("daemon stdout")?;
    let _guard = common::ChildGuard(child);
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let _port: u16 = line.trim().parse()?;

    // @step Then the workspace registry lists that project
    let registry: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        home.path().join(".fspec/workspaces.json"),
    )?)?;
    let canonical = std::fs::canonicalize(workspace_dir.path())?;
    assert!(
        registry["workspaces"]
            .as_array()
            .ok_or("workspaces array")?
            .iter()
            .any(|entry| entry["path"] == canonical.to_string_lossy().as_ref()),
        "{registry}"
    );
    Ok(())
}
