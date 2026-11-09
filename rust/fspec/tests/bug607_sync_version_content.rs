//! BUG-607: `fspec --sync-version` reinstalls the agent's doc and slash
//! command when their content is not what this fspec writes, not only when
//! the version string differs.
//!
//! Feature: spec/features/automatic-version-check-and-update-for-slash-command-files.feature
//!
//! Each test drives the real `fspec` binary in a temp project (HOME
//! redirected into the sandbox, FSPEC_AGENT cleared so the agent comes from
//! the project) and compares the files with a fresh `fspec init` in a second
//! sandbox.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_test_helpers::{setup_temp_directory, TestResult};

mod common;

use common::fspec_bin;

/// The running binary version, as fspec-core embeds it.
const CURRENT: &str = env!("CARGO_PKG_VERSION");
const DOC: &str = "spec/AGENTS.md";
const SLASH: &str = ".claude/commands/fspec.md";
const CONFIG: &str = r#"{"agent":"claude","tools":{"test":{"command":"cargo test"}}}"#;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_fspec(cwd: &Path, args: &[&str]) -> Result<Run, Box<dyn std::error::Error>> {
    let output = Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd)
        .env_remove("FSPEC_AGENT")
        .env_remove("FSPEC_CAPTURE_MODE")
        .output()?;
    Ok(Run {
        code: output.status.code().ok_or("fspec was killed by a signal")?,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// A project with a test command configured, then `fspec init --agent=claude`.
fn configured_claude_project(root: &Path) -> TestResult {
    fs::create_dir_all(root.join("spec"))?;
    fs::write(root.join("spec/fspec-config.json"), CONFIG)?;
    let init = run_fspec(root, &["init", "--agent=claude"])?;
    assert_eq!(init.code, 0, "fspec init failed: {}", init.stderr);
    Ok(())
}

/// What a fresh `fspec init --agent=claude` writes to `rel` in a project
/// configured the same way.
fn fresh_init(rel: &str) -> Result<String, Box<dyn std::error::Error>> {
    let fresh = setup_temp_directory()?;
    configured_claude_project(fresh.path())?;
    Ok(fs::read_to_string(fresh.path().join(rel))?)
}

fn read(root: &Path, rel: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(root.join(rel))?)
}

fn sync_current(root: &Path) -> Result<Run, Box<dyn std::error::Error>> {
    run_fspec(root, &["--sync-version", CURRENT])
}

#[test]
fn a_stale_doc_is_reinstalled_although_the_versions_match() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    configured_claude_project(root)?;
    // @step And spec/AGENTS.md holds instructions this fspec no longer writes
    fs::write(
        root.join(DOC),
        "Run fspec audit-coverage and fspec check.\n",
    )?;

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let run = sync_current(root)?;

    // @step Then spec/AGENTS.md is rewritten to exactly what "fspec init --agent=claude" writes
    assert_eq!(read(root, DOC)?, fresh_init(DOC)?);
    // @step And the output names spec/AGENTS.md as updated and tells the agent to restart, in a system-reminder
    assert!(run.stdout.contains("<system-reminder>"), "{}", run.stdout);
    assert!(run.stdout.contains(DOC), "{}", run.stdout);
    assert!(
        run.stdout
            .contains("Exit this conversation and start a new one. Run /fspec again."),
        "{}",
        run.stdout
    );
    // @step And it should exit with code 1
    assert_eq!(run.code, 1, "stderr: {}", run.stderr);
    Ok(())
}

#[test]
fn an_edited_slash_command_is_reinstalled_although_the_versions_match() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    configured_claude_project(root)?;
    // @step And .claude/commands/fspec.md was edited by hand
    let edited = format!("{}\nAlso run fspec check.\n", read(root, SLASH)?);
    fs::write(root.join(SLASH), edited)?;

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let run = sync_current(root)?;

    // @step Then .claude/commands/fspec.md is rewritten to exactly what "fspec init --agent=claude" writes
    assert_eq!(read(root, SLASH)?, fresh_init(SLASH)?);
    // @step And the output names .claude/commands/fspec.md as updated and tells the agent to restart
    assert!(run.stdout.contains(SLASH), "{}", run.stdout);
    assert!(run.stdout.contains("restart"), "{}", run.stdout);
    // @step And it should exit with code 1
    assert_eq!(run.code, 1, "stderr: {}", run.stderr);
    Ok(())
}

#[test]
fn a_deleted_doc_is_written_again_although_the_versions_match() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    configured_claude_project(root)?;
    // @step And spec/AGENTS.md was deleted
    fs::remove_file(root.join(DOC))?;

    // @step When the AI agent runs "fspec --sync-version" with the current version
    let run = sync_current(root)?;

    // @step Then spec/AGENTS.md exists again with exactly what "fspec init --agent=claude" writes
    assert_eq!(read(root, DOC)?, fresh_init(DOC)?);
    // @step And it should exit with code 1
    assert_eq!(run.code, 1, "stdout: {} stderr: {}", run.stdout, run.stderr);
    Ok(())
}

#[test]
fn files_that_match_what_this_fspec_writes_are_left_alone() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    configured_claude_project(root)?;
    let tracked = [DOC, SLASH, "spec/fspec-config.json"];
    let modified = |rel: &str| fs::metadata(root.join(rel)).and_then(|meta| meta.modified());
    let before = tracked
        .iter()
        .map(|rel| Ok((read(root, rel)?, modified(rel)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

    // @step When the AI agent runs "fspec --sync-version" with the current version twice
    let runs = [sync_current(root)?, sync_current(root)?];

    // @step Then neither run writes a file
    let after = tracked
        .iter()
        .map(|rel| Ok((read(root, rel)?, modified(rel)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    assert_eq!(before, after);
    // @step And both runs print nothing and exit with code 0
    for run in runs {
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (0, "", "")
        );
    }
    Ok(())
}
