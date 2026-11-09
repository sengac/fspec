//! BUG-599: codelet-git's tests never depend on the user's git config.
//!
//! Feature: spec/features/a-merge-commits-without-a-git-identity-and-git-tests-ignore-the-users-git-config.feature
//!
//! Source shape: every git process a codelet-git test spawns sets
//! GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1, so no test reads
//! the identity or any other setting from the machine it runs on.

use std::path::{Path, PathBuf};

use codelet_test_helpers::TestResult;

const SPAWN: &str = "Command::new(\"git\")";

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_sources(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The statement starting at byte `at`: up to the first `;` that closes it,
/// or the end of the expression block it is the tail of.
fn statement(source: &str, at: usize) -> &str {
    let mut depth = 0i64;
    for (offset, c) in source[at..].char_indices() {
        match c {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return &source[at..at + offset];
                }
            }
            ';' if depth == 0 => return &source[at..at + offset],
            _ => {}
        }
    }
    &source[at..]
}

/// Scenario: Git tests spawn git only through the isolating helper
#[test]
fn git_tests_spawn_git_only_through_the_isolating_helper() -> TestResult {
    // @step Given the codelet-git test sources
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut sources = Vec::new();
    rust_sources(&tests, &mut sources)?;
    sources.sort();
    assert!(sources.len() > 20, "found only {} sources", sources.len());

    // @step When they are scanned for processes spawning git
    let this_file = Path::new(file!())
        .file_name()
        .ok_or("this test has no file name")?;
    let mut spawns = 0usize;
    let mut offenders = Vec::new();
    for path in &sources {
        if path.file_name() == Some(this_file) {
            continue;
        }
        let source = std::fs::read_to_string(path)?;
        for (at, _) in source.match_indices(SPAWN) {
            spawns += 1;
            let spawn = statement(&source, at);
            let isolated = spawn.contains(".env(\"GIT_CONFIG_GLOBAL\", \"/dev/null\")")
                && spawn.contains(".env(\"GIT_CONFIG_NOSYSTEM\", \"1\")");
            if !isolated {
                let line = source[..at].matches('\n').count() + 1;
                let relative = path.strip_prefix(&tests).unwrap_or(path);
                offenders.push(format!("{}:{line}", relative.display()));
            }
        }
    }
    assert!(spawns > 10, "found only {spawns} git spawns");

    // @step Then every git process sets GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1
    assert!(
        offenders.is_empty(),
        "{} git spawns read the user's or the system's git config:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
    Ok(())
}
