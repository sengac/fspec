//! CLI surface for the `astgrep` subcommand on the standalone fspec Rust
//! binary — CLI-015.
//!
//! Feature: spec/features/fspec-astgrep-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

mod common;

use common::fspec_bin;

fn run_astgrep(cwd: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("astgrep");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let out = cmd.output().expect("spawn fspec astgrep");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn scenario_astgrep_runs_an_ast_search_and_prints_matches() {
    // @step Given a temp project root containing a Rust source file with a top-level `fn main() { ... }`
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join("src")).expect("mkdir src");
    std::fs::write(
        tmp.path().join("src").join("main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .expect("write main.rs");

    // @step When I run `fspec astgrep --pattern "fn $NAME($$$ARGS) { $$$BODY }" --lang rust --path src/` in that directory
    let (code, stdout, stderr) = run_astgrep(
        tmp.path(),
        &[
            "--pattern",
            "fn $NAME($$$ARGS) { $$$BODY }",
            "--lang",
            "rust",
            "--path",
            "src/",
        ],
    );

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "astgrep must exit 0 on matches; stderr={stderr}");

    // @step And stdout contains a match line in `file:line:column:text` format for the source file
    assert!(
        stdout.contains("main.rs:1:1:fn main() {"),
        "stdout must contain the file:line:column:text match line; got:\n{stdout}"
    );
}

#[test]
fn scenario_astgrep_requires_pattern_and_lang() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec astgrep` without a `--pattern` argument
    let (code, _stdout, stderr) = run_astgrep(tmp.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "missing --pattern must exit 1; stderr={stderr}");

    // @step And stderr mentions the missing required argument `--pattern`
    assert!(
        stderr.contains("--pattern"),
        "stderr must name the missing flag; got:\n{stderr}"
    );

    // @step And when I run `fspec astgrep --pattern "fn $NAME($$$ARGS) { $$$BODY }"` without a `--lang` argument
    let (code2, _stdout2, stderr2) = run_astgrep(
        tmp.path(),
        &["--pattern", "fn $NAME($$$ARGS) { $$$BODY }"],
    );

    // @step Then the command exits with code 1
    assert_eq!(code2, 1, "missing --lang must exit 1; stderr={stderr2}");

    // @step And stderr mentions the missing required argument `--lang`
    assert!(
        stderr2.contains("--lang"),
        "stderr must name the missing flag; got:\n{stderr2}"
    );
}

#[test]
fn scenario_astgrep_reports_an_invalid_pattern_to_stderr_and_exits_1() {
    // @step Given a temp project root containing at least one Rust source file
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(tmp.path().join("lib.rs"), "fn helper() {}\n").expect("write lib.rs");

    // @step When I run `fspec astgrep --pattern "fn $NAME() { $$$BODY } fn $NAME() { $$$BODY }" --lang rust --path lib.rs`
    let (code, _stdout, stderr) = run_astgrep(
        tmp.path(),
        &[
            "--pattern",
            "fn $NAME() { $$$BODY } fn $NAME() { $$$BODY }",
            "--lang",
            "rust",
            "--path",
            "lib.rs",
        ],
    );

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "invalid pattern must exit 1; stderr={stderr}");

    // @step And stderr contains "Error: Invalid AST pattern"
    assert!(
        stderr.contains("Error: Invalid AST pattern"),
        "stderr must contain the invalid-pattern message; got:\n{stderr}"
    );
}
