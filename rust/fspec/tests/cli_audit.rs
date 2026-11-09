//! AUDIT-003 — fspec audit on the command line; check and audit-coverage are
//! gone.
//!
//! Feature: spec/features/fspec-audit-is-the-one-deterministic-drift-check.feature

use std::path::Path;
use std::process::{Command, Output};

mod common;

use common::fspec_bin;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fspec(cwd: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .output()
}

#[test]
fn check_and_audit_coverage_are_no_longer_commands() -> TestResult {
    // @step Given the fspec command line
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    for args in [["check", "--verbose"], ["audit-coverage", "login"]] {
        // @step When the removed check and audit-coverage commands are parsed
        let output = fspec(root.path(), &args)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        // @step Then each is an unknown command
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stderr}");
        assert!(
            stderr.contains(&format!("error: unknown command '{}'", args[0])),
            "{args:?}: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn audit_prints_its_json_and_exit_code_on_the_command_line() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(
        root.path().join("spec/features/old.feature.coverage"),
        r#"{"scenarios":[]}"#,
    )?;
    let output = fspec(root.path(), &["audit", "--json"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let report: serde_json::Value = serde_json::from_str(stdout.trim())?;
    assert_eq!(report["exitCode"], 1, "{stdout}");
    assert_eq!(report["problems"][0]["code"], "orphan-coverage", "{stdout}");
    let unknown = fspec(root.path(), &["audit", "--area", "nonsense"])?;
    assert_eq!(unknown.status.code(), Some(2));
    Ok(())
}
