//! BUG-317: self-building shape tests reuse the ci-test build.
//! BUG-574: the rule covers cargo check and every test file of the workspace.
//!
//! Feature: spec/features/nested-build-tests-reuse-ci-test-profile.feature

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// This checker: its own sources quote the markers it looks for.
const CHECKER: &str = "sessions/tests/bug317_nested_build_profile.rs";

/// Every `.rs` file under `rust/<crate>/tests/`, relative to `rust/`, sorted.
fn workspace_test_files(root: &Path) -> std::io::Result<Vec<String>> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let tests = entry?.path().join("tests");
        if tests.is_dir()
            && tests
                .parent()
                .is_some_and(|c| c.join("Cargo.toml").is_file())
        {
            walk(&tests, &mut files)?;
        }
    }
    let mut rel: Vec<String> = files
        .iter()
        .filter_map(|f| f.strip_prefix(root).ok())
        .map(|f| f.to_string_lossy().replace('\\', "/"))
        .filter(|f| f != CHECKER)
        .collect();
    rel.sort();
    Ok(rel)
}

/// The `cargo <sub>` invocations of `src` that pass neither
/// `--profile ci-test` nor `--release`.
fn offending_subcommands(src: &str) -> Vec<&str> {
    // A shared `run_cargo` helper may inject the profile for its callers.
    let helper_injects = src.contains("fn run_cargo(") && src.contains("\"ci-test\"");
    let mut offenders = Vec::new();
    for stmt in nested_cargo_statements(src) {
        let Some(sub) = first_string_literal(stmt) else {
            continue;
        };
        if !matches!(sub, "build" | "clippy" | "test" | "check") {
            continue;
        }
        let via_helper = stmt.starts_with("run_cargo(") && helper_injects;
        if !(stmt.contains("\"ci-test\"") || stmt.contains("\"--release\"") || via_helper) {
            offenders.push(sub);
        }
    }
    offenders
}

/// Every nested cargo statement: from `Command::new(..cargo..)` to its
/// `.output()`, or a `run_cargo(&[ .. ])` call.
fn nested_cargo_statements(src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for marker in [
        "Command::new(env!(\"CARGO\"))",
        "Command::new(\"cargo\")",
        "run_cargo(&[",
    ] {
        let mut from = 0;
        while let Some(pos) = src[from..].find(marker) {
            let start = from + pos;
            let end_marker = if marker.starts_with("run_cargo") {
                "]);"
            } else {
                ".output()"
            };
            let end = src[start..]
                .find(end_marker)
                .map_or(src.len(), |e| start + e);
            out.push(&src[start..end]);
            from = start + marker.len();
        }
    }
    out
}

fn first_string_literal(stmt: &str) -> Option<&str> {
    let open = stmt.find('"')?;
    // skip the "CARGO"/"cargo" literal of Command::new(...)
    let rest = &stmt[open + 1..];
    let close = rest.find('"')?;
    let first = &rest[..close];
    if first == "CARGO" || first == "cargo" {
        let after = &rest[close + 1..];
        let o = after.find('"')?;
        let tail = &after[o + 1..];
        return tail.find('"').map(|c| &tail[..c]);
    }
    Some(first)
}

#[test]
fn nested_cargo_build_invocations_select_the_ci_test_profile() -> TestResult {
    // @step Given every test file under rust/*/tests
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("rust/ root")?;
    let files = workspace_test_files(root)?;
    assert!(
        files
            .iter()
            .any(|f| f == "graph/tests/codelet_graph_crate_lift.rs"),
        "the scan must reach every crate's tests: {} files",
        files.len()
    );

    // @step When every nested cargo invocation is inspected
    let mut offenders = Vec::new();
    for rel in &files {
        let src = std::fs::read_to_string(root.join(rel))?;
        for sub in offending_subcommands(&src) {
            offenders.push(format!("{rel}: cargo {sub}"));
        }
    }

    // @step Then each build, check, clippy or test invocation passes --profile ci-test or --release
    assert!(
        offenders.is_empty(),
        "nested cargo builds without --profile ci-test: {offenders:#?}"
    );
    Ok(())
}

#[test]
fn a_nested_cargo_call_without_a_profile_is_reported() {
    // @step Given a test source that runs cargo test without --profile
    let src = concat!(
        "let out = Command::new(env!(\"CARGO\"))\n",
        "    .args([\"test\", \"-p\", \"codelet-core\"])\n",
        "    .output();\n",
        "let ok = Command::new(env!(\"CARGO\"))\n",
        "    .args([\"check\", \"--profile\", \"ci-test\"])\n",
        "    .output();\n",
    );
    // @step When its nested cargo invocations are inspected
    let offenders = offending_subcommands(src);
    // @step Then the check reports the file and the "test" subcommand
    assert_eq!(offenders, vec!["test"]);
    let report = format!("{}: cargo {}", "x/tests/new_test.rs", offenders[0]);
    assert_eq!(report, "x/tests/new_test.rs: cargo test");
}
