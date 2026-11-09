//! BUG-548 — no test reads the machine's credential set.
//!
//! `ProviderManager::new()` and `Session::new(None)` pick the single
//! credentialed provider of the machine and refuse when several are
//! credentialed; Codex sessions need a Codex login. A test may use them only
//! when it points the file-based credential homes (FSPEC_HOME, CODEX_HOME) at
//! a test directory. Offline sessions select their provider explicitly, so no
//! test skips itself without a provider or picks the first credentialed one.
//!
//! Feature: spec/features/test-stub-provider-declares-its-limits.feature

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Calls whose result depends on the machine's credential set.
const MACHINE_CREDENTIAL_CALLS: [&str; 5] = [
    "ProviderManager::new()",
    "Session::new(None)",
    "Session::new(Some(\"codex\"))",
    "with_provider(\"codex\")",
    "with_provider_and_model(\"codex\"",
];

/// Credential homes a test must redirect before it uses one of those calls.
const CREDENTIAL_HOMES: [&str; 2] = ["\"FSPEC_HOME\"", "\"CODEX_HOME\""];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn is_unit_test_file(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            stem.starts_with("tests") || stem.ends_with("_tests") || stem.ends_with("_test")
        })
}

/// Every test source of every workspace crate: `tests/**/*.rs` plus unit-test
/// files under `src/` (`tests*.rs`, `*_tests.rs`, `*_test.rs`).
fn test_files() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(workspace_root())? {
        let crate_dir = entry?.path();
        if !crate_dir.join("Cargo.toml").is_file() {
            continue;
        }
        let tests = crate_dir.join("tests");
        if tests.is_dir() {
            rust_files(&tests, &mut files)?;
        }
        let src = crate_dir.join("src");
        if src.is_dir() {
            let mut sources = Vec::new();
            rust_files(&src, &mut sources)?;
            files.extend(sources.into_iter().filter(|path| is_unit_test_file(path)));
        }
    }
    // This guard names the calls it looks for.
    files.retain(|path| {
        !path.ends_with("providers/tests/bug548_tests_ignore_machine_credentials.rs")
    });
    files.sort();
    Ok(files)
}

/// The file's code lines, without line comments.
fn code_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect()
}

/// The file plus the modules it pulls in with `#[path = "..."]`, which is
/// where shared fixtures set the credential homes.
fn source_with_path_modules(path: &Path) -> std::io::Result<String> {
    let mut source = std::fs::read_to_string(path)?;
    let directory = path.parent().unwrap_or(Path::new("."));
    let modules: Vec<PathBuf> = source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("#[path = \"")?;
            let (module, _) = rest.split_once('"')?;
            Some(directory.join(module))
        })
        .collect();
    for module in modules {
        if let Ok(text) = std::fs::read_to_string(module) {
            source.push('\n');
            source.push_str(&text);
        }
    }
    Ok(source)
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn offline_test_sessions_select_their_provider_explicitly_and_run() -> TestResult {
    // @step Given the test files of every crate
    let files = test_files()?;
    assert!(files.len() > 100, "the guard found too few test files");

    // @step When the guard scans them for tests that skip without a provider or pick the first credentialed provider in a loop
    let mut offenders = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path)?;
        let lines = code_lines(&source);
        for (index, line) in lines.iter().enumerate() {
            let skips = line.contains("Skipping test")
                && (line.contains("provider") || line.contains("credential"));
            let first_credentialed = line.contains("for ")
                && line.contains(" in [")
                && line.contains("\"claude\"")
                && lines
                    .iter()
                    .skip(index + 1)
                    .take(2)
                    .any(|next| next.contains("with_provider("));
            if skips || first_credentialed {
                offenders.push(format!("{}: {}", relative(path), line.trim()));
            }
        }
    }

    // @step Then none is found, because offline sessions select their provider explicitly and run
    assert!(
        offenders.is_empty(),
        "tests that skip without a provider or pick the machine's provider:\n{}",
        offenders.join("\n")
    );
    Ok(())
}

#[test]
fn the_source_guard_reports_tests_that_read_the_machines_credential_set() -> TestResult {
    // @step Given the test files of every crate
    let files = test_files()?;
    assert!(files.len() > 100, "the guard found too few test files");

    // @step When the guard scans them for ProviderManager::new(), Session::new(None) and Codex sessions
    let mut offenders = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path)?;
        let calls: Vec<&str> = MACHINE_CREDENTIAL_CALLS
            .into_iter()
            .filter(|call| code_lines(&source).iter().any(|line| line.contains(call)))
            .collect();
        if calls.is_empty() {
            continue;
        }
        let context = source_with_path_modules(path)?;
        let isolated = CREDENTIAL_HOMES
            .iter()
            .all(|home| code_lines(&context).iter().any(|line| line.contains(home)));
        if !isolated {
            offenders.push(format!("{}: {}", relative(path), calls.join(", ")));
        }
    }

    // @step Then every such use is in a file that points FSPEC_HOME and CODEX_HOME at a test directory
    assert!(
        offenders.is_empty(),
        "tests that read the machine's credential set (redirect FSPEC_HOME and CODEX_HOME, \
         or select the provider explicitly):\n{}",
        offenders.join("\n")
    );
    Ok(())
}
