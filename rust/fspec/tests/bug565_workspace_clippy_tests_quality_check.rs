//! BUG-565 — the quality checks lint every workspace test target.
//!
//! An unused import in a test file broke `cargo clippy -p codelet-fspec
//! --tests -- -D warnings` without anyone noticing, because the documented
//! quality checks only ran `cargo clippy -p codelet-fspec`, which never lints
//! test targets. AGENTS.md now lists the workspace-wide `--tests` run.
//!
//! Feature: spec/features/workspace-clippy-tests-quality-check.feature

use std::fs;
use std::path::PathBuf;

/// The exact command every maintainer runs before committing.
const WORKSPACE_CLIPPY_TESTS: &str = "cargo clippy --workspace --tests -- -D warnings";

/// The body of the AGENTS.md section whose heading ends with `title`, up to the
/// next heading of the same or a higher level.
fn section(markdown: &str, title: &str) -> Option<String> {
    let mut lines = markdown.lines();
    let level = loop {
        let line = lines.next()?;
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if hashes > 0 && line.trim_end().ends_with(title) {
            break hashes;
        }
    };
    let body: Vec<&str> = lines
        .take_while(|line| {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            hashes == 0 || hashes > level
        })
        .collect();
    Some(body.join("\n"))
}

// Scenario: The documented quality checks lint every workspace test target
#[test]
fn documented_quality_checks_lint_every_workspace_test_target() -> Result<(), String> {
    // @step Given the "Quality Check Integration" section of AGENTS.md
    let agents_md = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../AGENTS.md");
    let text = fs::read_to_string(&agents_md)
        .map_err(|e| format!("cannot read {}: {e}", agents_md.display()))?;
    let quality = section(&text, "Quality Check Integration")
        .ok_or("AGENTS.md has no 'Quality Check Integration' section")?;

    // @step When a maintainer reads the quality checks to run before committing
    let commands: Vec<&str> = quality
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|line| line.starts_with("cargo "))
        .collect();

    // @step Then the section lists "cargo clippy --workspace --tests -- -D warnings"
    if commands.contains(&WORKSPACE_CLIPPY_TESTS) {
        Ok(())
    } else {
        Err(format!(
            "AGENTS.md 'Quality Check Integration' must list `{WORKSPACE_CLIPPY_TESTS}`; it lists {commands:?}"
        ))
    }
}
