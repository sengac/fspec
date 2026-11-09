//! RPC-446: per-daemon local discovery files.
//!
//! Feature: spec/features/per-daemon-discovery-files.feature

mod common;

#[allow(dead_code)]
#[path = "../src/common.rs"]
mod fspec_common;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// BUG-595: the plain loopback listener these entries advertise.
fn loopback() -> fspec_common::DaemonListener {
    fspec_common::DaemonListener::loopback()
}

/// A live process other than this test (so two distinct live pids exist).
fn live_helper_process() -> std::io::Result<Child> {
    Command::new("sleep").arg("30").spawn()
}

fn entry_json(dir: &Path, pid: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join(format!("{pid}.json")),
    )?)?)
}

#[test]
fn a_daemon_entry_is_written_per_process() -> TestResult {
    // @step Given an empty daemons directory
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let alpha = tempfile::tempdir()?;
    let beta = tempfile::tempdir()?;
    let mut helper = live_helper_process()?;
    let (pid_a, pid_b) = (std::process::id(), helper.id());

    // @step When daemon entries are written for two live processes with workspaces "alpha" and "beta"
    fspec_common::write_daemon_entry(
        &dir,
        4001,
        pid_a,
        alpha.path(),
        SystemTime::now(),
        &loopback(),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4002,
        pid_b,
        beta.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step Then the directory holds one <pid>.json per process with its port and workspace
    let a = entry_json(&dir, pid_a)?;
    let b = entry_json(&dir, pid_b)?;
    helper.kill()?;
    assert_eq!(a["port"], 4001);
    assert_eq!(b["port"], 4002);
    let canonical_beta = std::fs::canonicalize(beta.path())?;
    assert_eq!(b["workspace"], canonical_beta.to_string_lossy().as_ref());
    Ok(())
}

#[test]
fn listing_returns_live_daemons_and_prunes_stale_entries() -> TestResult {
    // @step Given a daemons directory with an entry for the current process and one for a dead pid
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let workspace = tempfile::tempdir()?;
    let mut dead = live_helper_process()?;
    let dead_pid = dead.id();
    dead.kill()?;
    dead.wait()?;
    fspec_common::write_daemon_entry(
        &dir,
        4101,
        std::process::id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4102,
        dead_pid,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When the live daemons are listed
    let live = fspec_common::list_live_daemons(&dir)?;

    // @step Then only the current process's daemon is returned
    assert_eq!(live.len(), 1, "{live:?}");
    assert_eq!(live[0].pid, std::process::id());
    assert_eq!(live[0].port, 4101);

    // @step And the dead pid's file has been removed
    assert!(!dir.join(format!("{dead_pid}.json")).exists());
    Ok(())
}

#[test]
fn removing_an_entry_deletes_only_that_daemons_file() -> TestResult {
    // @step Given daemon entries for two live processes
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let workspace = tempfile::tempdir()?;
    let mut helper = live_helper_process()?;
    let (pid_a, pid_b) = (std::process::id(), helper.id());
    fspec_common::write_daemon_entry(
        &dir,
        4201,
        pid_a,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4202,
        pid_b,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When one daemon removes its entry on shutdown
    fspec_common::remove_daemon_entry(&dir, pid_b);

    // @step Then only the other daemon's file remains
    helper.kill()?;
    assert!(dir.join(format!("{pid_a}.json")).exists());
    assert!(!dir.join(format!("{pid_b}.json")).exists());
    Ok(())
}

#[test]
fn a_running_daemon_registers_itself_beside_daemon_json() -> TestResult {
    // @step Given a runtime directory for discovery files
    let (workspace_dir, _) = common::make_workspace(&[("AUTH-001", "Login", "backlog")]);
    let home = tempfile::tempdir()?;
    let runtime = tempfile::tempdir()?;

    // @step When fspec daemon starts in a workspace
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
    let pid = child.id();
    let stdout = child.stdout.take().ok_or("daemon stdout")?;
    let _guard = common::ChildGuard(child);
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let port: u16 = line.trim().parse()?;

    // @step Then daemons/<pid>.json names the daemon's port and workspace
    let fspec_dir = runtime.path().join("fspec");
    let entry_path = fspec_dir.join("daemons").join(format!("{pid}.json"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(entry_path.exists() && fspec_dir.join("daemon.json").exists())
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    let entry = entry_json(&fspec_dir.join("daemons"), pid)?;
    assert_eq!(entry["port"], port);
    let canonical = std::fs::canonicalize(workspace_dir.path())?;
    assert_eq!(entry["workspace"], canonical.to_string_lossy().as_ref());

    // @step And daemon.json still points at the daemon
    let pointer: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fspec_dir.join("daemon.json"))?)?;
    assert_eq!(pointer["pid"], pid);
    Ok(())
}

#[test]
fn fspec_status_lists_every_live_local_daemon() -> TestResult {
    // @step Given a running fspec daemon advertised in the daemons directory
    let (workspace_dir, _) = common::make_workspace(&[("AUTH-001", "Login", "backlog")]);
    let home = tempfile::tempdir()?;
    let runtime = tempfile::tempdir()?;
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
    let pid = child.id();
    let stdout = child.stdout.take().ok_or("daemon stdout")?;
    let _guard = common::ChildGuard(child);
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let port: u16 = line.trim().parse()?;
    let entry_path = runtime
        .path()
        .join("fspec/daemons")
        .join(format!("{pid}.json"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !entry_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }

    // @step When fspec status is run against the same runtime directory
    let output = Command::new(common::fspec_bin())
        .arg("status")
        .env("HOME", home.path())
        .env("FSPEC_USER_DIR", home.path().join(".fspec"))
        .env("XDG_RUNTIME_DIR", runtime.path())
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);

    // @step Then the output lists the daemon with its pid, port and workspace
    let canonical = std::fs::canonicalize(workspace_dir.path())?;
    let expected = format!("pid={pid} port={port} workspace={}", canonical.display());
    assert!(text.contains("local daemons: 1"), "{text}");
    assert!(text.contains(&expected), "missing {expected:?} in:\n{text}");
    Ok(())
}

#[path = "rpc446_per_daemon_discovery/default_pointer.rs"]
mod default_pointer;
