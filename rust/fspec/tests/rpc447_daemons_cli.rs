//! RPC-447: managing saved daemons from the CLI.
//!
//! Feature: spec/features/daemons-cli.feature

mod common;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Output, Stdio};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fspec(home: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(common::fspec_bin())
        .args(args)
        .env("HOME", home)
        .env("FSPEC_USER_DIR", home.join(".fspec"))
        .env_remove("FSPEC_DAEMON_TOKEN")
        .output()
}

#[test]
fn adding_and_listing_saved_daemons_masks_the_token() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;

    // @step When "fspec daemons add work wss://fspec.example.com --token s3cret" and "fspec daemons list" are run
    let add = fspec(
        home.path(),
        &[
            "daemons",
            "add",
            "work",
            "wss://fspec.example.com",
            "--token",
            "s3cret",
        ],
    )?;
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let list = fspec(home.path(), &["daemons", "list"])?;
    let text = String::from_utf8_lossy(&list.stdout);

    // @step Then the list shows "work" and its URL but not the token
    assert!(
        text.contains("work") && text.contains("wss://fspec.example.com"),
        "{text}"
    );
    assert!(!text.contains("s3cret"), "the token must be masked: {text}");
    Ok(())
}

#[test]
fn removing_a_saved_daemon_from_the_cli() -> TestResult {
    // @step Given a saved daemon "work"
    let home = tempfile::tempdir()?;
    fspec(
        home.path(),
        &[
            "daemons",
            "add",
            "work",
            "wss://fspec.example.com",
            "--token",
            "s3cret",
        ],
    )?;

    // @step When "fspec daemons remove work" is run
    let remove = fspec(home.path(), &["daemons", "remove", "work"])?;
    assert!(
        remove.status.success(),
        "{}",
        String::from_utf8_lossy(&remove.stderr)
    );

    // @step Then "fspec daemons list" no longer shows "work"
    let list = fspec(home.path(), &["daemons", "list"])?;
    assert!(!String::from_utf8_lossy(&list.stdout).contains("work"));
    Ok(())
}

#[test]
fn status_connects_to_a_token_protected_daemon_by_saved_name() -> TestResult {
    // @step Given a daemon started with FSPEC_DAEMON_TOKEN "s3cret" and saved as "local" with that token
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
        .env("FSPEC_DAEMON_TOKEN", "s3cret")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("daemon stdout")?;
    let _guard = common::ChildGuard(child);
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let url = format!("ws://127.0.0.1:{}", line.trim());
    let add = fspec(
        home.path(),
        &["daemons", "add", "local", &url, "--token", "s3cret"],
    )?;
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    // @step When "fspec status --connect local" is run
    let status = fspec(home.path(), &["status", "--connect", "local"])?;

    // @step Then the daemon reports alive
    let text = String::from_utf8_lossy(&status.stdout);
    assert!(
        text.contains("fspec daemon: alive"),
        "stdout={text} stderr={}",
        String::from_utf8_lossy(&status.stderr)
    );

    // @step And "fspec status" with a raw URL and no token fails
    let raw = fspec(home.path(), &["status", "--connect", &url])?;
    assert!(!raw.status.success(), "an unauthenticated status must fail");
    Ok(())
}
