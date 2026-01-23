#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration tests for codelet CLI
//!
//! Following BDD principles for CLI testing:
//! - Uses assert_cmd for binary invocation
//! - Uses predicates for output assertions

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Get a command for the codelet binary
fn codelet_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("codelet")
}

#[test]
fn test_help_output() {
    codelet_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("CLI interface for codelet"));
}

#[test]
fn test_version_output() {
    codelet_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_exec_subcommand_help() {
    codelet_cmd()
        .args(["exec", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("non-interactive mode"));
}

#[test]
fn test_completion_generation() {
    codelet_cmd()
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_config_path() {
    codelet_cmd()
        .args(["config", "--path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config"));
}
