//! BUG-289: daemon.json (the default-daemon pointer) is never taken from a
//! live daemon, and a daemon that shuts down hands it to another live one.
use super::*;

fn pointer_pid(pointer: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let body: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(pointer)?)?;
    body["pid"]
        .as_u64()
        .ok_or_else(|| "daemon.json has no pid".into())
}

/// A pid that belonged to a process which has exited.
fn dead_pid() -> Result<u32, Box<dyn std::error::Error>> {
    let mut child = live_helper_process()?;
    let pid = child.id();
    child.kill()?;
    child.wait()?;
    Ok(pid)
}

#[test]
fn a_second_daemon_does_not_replace_a_live_daemons_daemon_json() -> TestResult {
    // @step Given daemon.json points at a live daemon
    let root = tempfile::tempdir()?;
    let pointer = root.path().join("daemon.json");
    let workspace = tempfile::tempdir()?;
    let mut first = live_helper_process()?;
    fspec_common::write_daemon_json(
        &pointer,
        4101,
        first.id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When another daemon starts and claims the default daemon.json
    let claimed = fspec_common::claim_default_pointer(
        &pointer,
        4102,
        std::process::id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step Then daemon.json still points at the first daemon
    let owner = pointer_pid(&pointer)?;
    first.kill()?;
    assert_eq!(owner, u64::from(first.id()));

    // @step And the second daemon reports that it did not take the default
    assert!(!claimed);
    Ok(())
}

#[test]
fn a_dead_daemons_daemon_json_is_replaced() -> TestResult {
    // @step Given daemon.json points at a process that is no longer running
    let root = tempfile::tempdir()?;
    let pointer = root.path().join("daemon.json");
    let workspace = tempfile::tempdir()?;
    fspec_common::write_daemon_json(
        &pointer,
        4201,
        dead_pid()?,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When a daemon starts and claims the default daemon.json
    let claimed = fspec_common::claim_default_pointer(
        &pointer,
        4202,
        std::process::id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step Then daemon.json points at the new daemon
    assert!(claimed);
    assert_eq!(pointer_pid(&pointer)?, u64::from(std::process::id()));
    Ok(())
}

#[test]
fn shutting_down_hands_daemon_json_to_another_live_daemon() -> TestResult {
    // @step Given daemon.json points at this daemon and another live daemon is listed in the daemons directory
    let root = tempfile::tempdir()?;
    let pointer = root.path().join("daemon.json");
    let dir = root.path().join("daemons");
    let workspace = tempfile::tempdir()?;
    let mut other = live_helper_process()?;
    let me = std::process::id();
    fspec_common::write_daemon_json(
        &pointer,
        4301,
        me,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4301,
        me,
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4302,
        other.id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When this daemon shuts down and releases the default daemon.json
    fspec_common::remove_daemon_entry(&dir, me);
    fspec_common::release_default_pointer(&pointer, &dir, me)?;

    // @step Then daemon.json points at the other live daemon
    let owner = pointer_pid(&pointer)?;
    let body: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pointer)?)?;
    other.kill()?;
    assert_eq!(owner, u64::from(other.id()));
    assert_eq!(body["port"], 4302);
    Ok(())
}

#[test]
fn shutting_down_leaves_another_daemons_daemon_json_alone() -> TestResult {
    // @step Given daemon.json points at another live daemon
    let root = tempfile::tempdir()?;
    let pointer = root.path().join("daemon.json");
    let dir = root.path().join("daemons");
    let workspace = tempfile::tempdir()?;
    let mut other = live_helper_process()?;
    fspec_common::write_daemon_json(
        &pointer,
        4401,
        other.id(),
        workspace.path(),
        SystemTime::now(),
        &loopback(),
    )?;

    // @step When this daemon shuts down and releases the default daemon.json
    fspec_common::release_default_pointer(&pointer, &dir, std::process::id())?;

    // @step Then daemon.json still points at the other daemon
    let owner = pointer_pid(&pointer)?;
    other.kill()?;
    assert_eq!(owner, u64::from(other.id()));
    Ok(())
}
