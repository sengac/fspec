//! TUI-123 — concurrent `fspec daemons add` processes never lose or corrupt
//! saved daemons.
//!
//! Feature: spec/features/daemon-management-on-par-with-workspaces.feature

mod common;

use std::process::{Command, Stdio};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn concurrent_fspec_daemons_add_processes_lose_nothing() -> TestResult {
    // @step Given an empty fspec home
    let home = tempfile::tempdir()?;

    // @step When 16 "fspec daemons add" processes run at the same time with different names
    let children = (0..16)
        .map(|index| {
            Command::new(common::fspec_bin())
                .args([
                    "daemons",
                    "add",
                    &format!("d{index}"),
                    &format!("ws://127.0.0.1:{}", 6000 + index),
                ])
                .env("HOME", home.path())
                .env_remove("FSPEC_DAEMON_TOKEN")
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut failures = Vec::new();
    for child in children {
        let output = child.wait_with_output()?;
        if !output.status.success() {
            failures.push(String::from_utf8_lossy(&output.stderr).into_owned());
        }
    }

    // @step Then daemons.json lists all 16 daemons
    assert!(failures.is_empty(), "{failures:?}");
    let path = home.path().join(".fspec/daemons.json");
    let mut names: Vec<String> = codelet_common::saved_daemons::list(&path)?
        .into_iter()
        .map(|daemon| daemon.name)
        .collect();
    names.sort();
    let mut expected: Vec<String> = (0..16).map(|index| format!("d{index}")).collect();
    expected.sort();
    assert_eq!(names, expected);

    // @step And daemons.json is mode 0600 and no temporary file is left
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "daemons.json mode was {mode:o}");
    }
    let leftovers: Vec<String> = std::fs::read_dir(home.path().join(".fspec"))?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    Ok(())
}
