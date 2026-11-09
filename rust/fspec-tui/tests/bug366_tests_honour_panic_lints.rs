//! BUG-366 — fspec-tui tests honour the workspace unwrap, expect and panic
//! lints.
//!
//! Feature: spec/features/fspec-tui-tests-honour-the-workspace-panic-lints.feature

mod common; // BUG-487: TaskOutcome lives in tests/common
use std::path::{Path, PathBuf};

use codelet_test_helpers::TestResult;

/// The lints the workspace denies and tests must not opt out of.
const LINTS: [&str; 3] = ["unwrap_used", "expect_used", "panic"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Every Rust file under rust/fspec-tui/tests with its source.
fn sources() -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"),
        &mut files,
    )?;
    files
        .into_iter()
        .map(|path| Ok((path.clone(), std::fs::read_to_string(&path)?)))
        .collect()
}

/// The `allow(...)` attributes opened by `opener` (`#![` or `#[`), as the
/// text between the parentheses; built at runtime so this file does not
/// match itself.
fn allows<'a>(source: &'a str, opener: &str) -> Vec<&'a str> {
    let needle = format!("{opener}allow(");
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(&needle) {
        let after = &rest[start + needle.len()..];
        let Some(end) = after.find(")]") else {
            break;
        };
        found.push(&after[..end]);
        rest = &after[end..];
    }
    found
}

fn names_a_lint(allow: &str) -> bool {
    LINTS.iter().any(|lint| {
        allow
            .split([',', ' ', '\n'])
            .any(|part| part == format!("clippy::{lint}"))
    })
}

#[test]
fn no_fspec_tui_test_file_or_helper_module_suppresses_the_lints() -> TestResult {
    // @step Given every Rust file under rust/fspec-tui/tests
    let sources = sources()?;
    // @step When their file and module level allow attributes are read
    let offenders: Vec<String> = sources
        .iter()
        .filter(|(_, source)| allows(source, "#![").into_iter().any(names_a_lint))
        .map(|(path, _)| path.display().to_string())
        .collect();
    // @step Then none allows clippy::unwrap_used, clippy::expect_used or clippy::panic
    assert!(
        offenders.is_empty(),
        "file-level lint suppressions: {offenders:#?}"
    );
    Ok(())
}

#[test]
fn a_deliberate_panic_carries_a_scoped_reasoned_allow() -> TestResult {
    // @step Given every Rust file under rust/fspec-tui/tests
    let sources = sources()?;
    // @step When an item-level allow of clippy::panic is found
    let unreasoned: Vec<String> = sources
        .iter()
        .flat_map(|(path, source)| {
            allows(source, "#[")
                .into_iter()
                .filter(|allow| names_a_lint(allow) && !allow.contains("reason"))
                .map(move |allow| format!("{}: {allow}", path.display()))
        })
        .collect();
    // @step Then it states its reason
    assert!(
        unreasoned.is_empty(),
        "lint allows without a reason: {unreasoned:#?}"
    );
    Ok(())
}

#[test]
fn no_fspec_tui_test_drops_a_tasks_join_error() -> TestResult {
    // @step Given every Rust file under rust/fspec-tui/tests
    let sources = sources()?;
    // @step When awaited tasks are inspected
    let prefix = ["let", "_", "="].join(" ");
    let dropped: Vec<String> = sources
        .iter()
        .flat_map(|(path, source)| {
            source.lines().enumerate().filter_map(|(index, line)| {
                let line = line.trim();
                let awaited = line
                    .strip_prefix(&prefix)
                    .and_then(|rest| rest.trim().strip_suffix(".await;"))
                    .is_some_and(|task| task.chars().all(|c| c.is_alphanumeric() || c == '_'));
                awaited.then(|| format!("{}:{}: {line}", path.display(), index + 1))
            })
        })
        .collect();
    // @step Then none is awaited as `let _ = <task>.await`
    assert!(dropped.is_empty(), "dropped join errors: {dropped:#?}");
    Ok(())
}

#[tokio::test]
async fn a_task_the_app_cancelled_does_not_fail_a_test() -> TestResult {
    use crate::common::TaskOutcome;

    // @step Given every Rust file under rust/fspec-tui/tests
    let sources = sources()?;
    // @step When the App's pending tasks are drained
    let drain = ["next_pending", "task()"].join("_");
    let outcome = ["completed", "or", "cancelled"].join("_");
    let mut bare = Vec::new();
    for (path, source) in &sources {
        let lines: Vec<&str> = source.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(&drain) {
                continue;
            }
            let join = lines
                .iter()
                .enumerate()
                .skip(index + 1)
                .take(3)
                .find(|(_, line)| line.contains(".await"));
            if let Some((at, join)) = join {
                let statement: String = lines[at..]
                    .iter()
                    .take_while(|line| !line.contains(';'))
                    .chain(lines[at..].iter().find(|line| line.contains(';')))
                    .copied()
                    .collect();
                if !statement.contains(&outcome) {
                    bare.push(format!("{}:{}: {}", path.display(), at + 1, join.trim()));
                }
            }
        }
    }
    // @step Then each join goes through completed_or_cancelled, so a task the App aborted passes and a panicked task fails
    assert!(
        bare.is_empty(),
        "pending-task joins without {outcome}: {bare:#?}"
    );
    let aborted = tokio::spawn(std::future::pending::<()>());
    aborted.abort();
    aborted.await.completed_or_cancelled()?;
    let panicked = tokio::spawn(async {
        #[allow(
            clippy::panic,
            reason = "the scenario needs a task that really panicked"
        )]
        {
            panic!("boom")
        }
    });
    assert!(panicked.await.completed_or_cancelled().is_err());
    Ok(())
}

#[test]
fn no_fspec_tui_test_recovers_a_poisoned_lock() -> TestResult {
    // @step Given every Rust file under rust/fspec-tui/tests
    let sources = sources()?;
    // @step When their lock calls are inspected
    let recovery = ["PoisonError", "into_inner"].join("::");
    let recovered: Vec<String> = sources
        .iter()
        .filter(|(path, _)| path.file_name() != Path::new(file!()).file_name())
        .flat_map(|(path, source)| {
            source
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(&recovery))
                .map(move |(index, line)| {
                    format!("{}:{}: {}", path.display(), index + 1, line.trim())
                })
        })
        .collect();
    // @step Then none recovers a poisoned lock with PoisonError::into_inner
    assert!(
        recovered.is_empty(),
        "poisoned-lock recoveries: {recovered:#?}"
    );
    Ok(())
}
