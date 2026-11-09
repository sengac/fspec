//! WT-015: isolated worktrees share the main checkout's board through one
//! state-root resolver.
//!
//! Feature: spec/features/isolated-worktrees-share-the-main-checkouts-board-through-one-state-root-resolver.feature
//!
//! Source shape: every spec path in the workspace's non-test code is built
//! by codelet-common's path module, so no command can read one root and
//! write another (BUG-523) or bypass the state root (WT-015).

use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The one module allowed to build spec paths.
const PATH_MODULE: &str = "common/src/spec_paths.rs";

/// Code that is not compiled into a non-test build: test files, test
/// directories and `#[cfg(test)]` items.
fn is_test_file(relative: &Path) -> bool {
    let name = relative
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
        || relative
            .components()
            .any(|c| matches!(c.as_os_str().to_str(), Some("tests" | "test_support")))
}

/// `source` with every `#[cfg(test)]` item removed. A small lexer keeps
/// braces inside strings, raw strings, chars and comments from counting.
fn without_cfg_test_items(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut kept = String::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() != "#[cfg(test)]" {
            kept.push_str(lines[i]);
            kept.push('\n');
            i += 1;
            continue;
        }
        // Skip further attributes, then the item itself.
        i += 1;
        while i < lines.len() && lines[i].trim_start().starts_with("#[") {
            i += 1;
        }
        let mut depth = 0i64;
        let mut opened = false;
        let mut lexer = Lexer::default();
        while i < lines.len() {
            let line = lines[i];
            i += 1;
            for delta in lexer.braces(line) {
                depth += delta;
                opened |= delta > 0;
            }
            if (opened && depth <= 0) || (!opened && line.trim_end().ends_with(';')) {
                break;
            }
        }
    }
    kept
}

#[derive(Default)]
struct Lexer {
    /// Inside a string; `Some(n)` for a raw string closed by `"` + n `#`.
    string: Option<Option<usize>>,
    block_comment: usize,
}

impl Lexer {
    /// +1 / -1 for every brace of `line` outside strings, chars and comments.
    fn braces(&mut self, line: &str) -> Vec<i64> {
        let chars: Vec<char> = line.chars().collect();
        let mut out = Vec::new();
        let mut k = 0;
        while k < chars.len() {
            let c = chars[k];
            let next = chars.get(k + 1).copied();
            if self.block_comment > 0 {
                if c == '*' && next == Some('/') {
                    self.block_comment -= 1;
                    k += 1;
                } else if c == '/' && next == Some('*') {
                    self.block_comment += 1;
                    k += 1;
                }
            } else if let Some(raw) = self.string {
                match raw {
                    None if c == '\\' => k += 1,
                    None if c == '"' => self.string = None,
                    Some(hashes)
                        if c == '"'
                            && chars[k + 1..]
                                .iter()
                                .take(hashes)
                                .filter(|h| **h == '#')
                                .count()
                                == hashes =>
                    {
                        self.string = None;
                        k += hashes;
                    }
                    _ => {}
                }
            } else if c == '/' && next == Some('/') {
                break;
            } else if c == '/' && next == Some('*') {
                self.block_comment += 1;
                k += 1;
            } else if c == 'r' && matches!(next, Some('"' | '#')) {
                let hashes = chars[k + 1..].iter().take_while(|h| **h == '#').count();
                if chars.get(k + 1 + hashes) == Some(&'"') {
                    self.string = Some(Some(hashes));
                    k += 1 + hashes;
                }
            } else if c == '"' {
                self.string = Some(None);
            } else if c == '\'' {
                // A char literal ('{', '\'', '\u{7b}'); a lifetime has no
                // closing quote within the next few chars.
                if next == Some('\\') {
                    if let Some(end) = chars[k + 2..].iter().position(|q| *q == '\'') {
                        k += 2 + end;
                    }
                } else if chars.get(k + 2) == Some(&'\'') {
                    k += 2;
                }
            } else if c == '{' {
                out.push(1);
            } else if c == '}' {
                out.push(-1);
            }
            k += 1;
        }
        out
    }
}

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

/// The workspace directory (rust/).
fn workspace() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("codelet-common has no parent directory")?
        .to_path_buf())
}

/// Every non-test Rust source of the workspace's crates as (path relative
/// to the workspace, source without `#[cfg(test)]` items), sorted by path.
fn non_test_sources() -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let workspace = workspace()?;
    let mut sources = Vec::new();
    for crate_dir in std::fs::read_dir(&workspace)? {
        let crate_dir = crate_dir?.path();
        let src = crate_dir.join("src");
        let name = crate_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string());
        if src.is_dir() && !matches!(name.as_deref(), Some("target" | "patches")) {
            rust_sources(&src, &mut sources)?;
        }
    }
    sources.sort();
    assert!(sources.len() > 500, "found only {} sources", sources.len());
    let mut out = Vec::new();
    for path in sources {
        let relative = path.strip_prefix(&workspace)?.to_path_buf();
        if is_test_file(&relative) {
            continue;
        }
        let source = without_cfg_test_items(&std::fs::read_to_string(&path)?);
        out.push((relative, source));
    }
    Ok(out)
}

#[test]
fn no_spec_path_is_built_outside_the_path_module() -> TestResult {
    // @step Given the workspace's non-test Rust sources
    let sources = non_test_sources()?;

    // @step When they are scanned for joins of "spec" and "spec/..." paths
    let mut offenders = Vec::new();
    for (relative, source) in &sources {
        for line in source.lines() {
            let code = line.split("//").next().unwrap_or_default();
            if code.contains(".join(\"spec\")") || code.contains(".join(\"spec/") {
                offenders.push(format!("{}: {}", relative.display(), line.trim()));
            }
        }
    }

    // @step Then the only matches are in codelet-common's spec path module
    let outside: Vec<&String> = offenders
        .iter()
        .filter(|o| !o.starts_with(&format!("{PATH_MODULE}:")))
        .collect();
    assert!(
        outside.is_empty(),
        "{} spec path joins outside {PATH_MODULE}:\n{}",
        outside.len(),
        outside
            .iter()
            .map(|o| o.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

/// Calls that resolve a project's state root and can fail on a broken
/// `.fspec/state-root` pointer: codelet-common's resolver and fspec-core's
/// `io::spec_paths` wrappers.
const RESOLUTION_CALLS: &[&str] = &[
    "ProjectRoots::resolve(",
    "ProjectRoots::of_content_root(",
    "read_state_root_pointer(",
    "lock_dir_for(",
    "spec_paths::roots(",
    "spec_paths::state_spec_dir(",
    "spec_paths::work_units(",
    "spec_paths::epics(",
    "spec_paths::prefixes(",
    "spec_paths::tags(",
    "spec_paths::tags_md(",
    "spec_paths::foundation(",
    "spec_paths::foundation_draft(",
    "spec_paths::foundation_md(",
    "spec_paths::attachments_dir(",
    "spec_paths::config(",
    "spec_paths::schedules(",
    "spec_paths::virtual_hooks_dir(",
];

/// `code` with every `//` comment removed (line by line).
fn without_line_comments(code: &str) -> String {
    code.lines()
        .map(|line| match line.find("//") {
            Some(at) if !line[..at].contains('"') => &line[..at],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The index just past the parenthesis that closes the one opened at
/// `open` (`chars[open] == '('`), or `None` when it never closes.
fn close_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (k, c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn skip_whitespace(chars: &[char], mut at: usize) -> usize {
    while at < chars.len() && chars[at].is_whitespace() {
        at += 1;
    }
    at
}

fn starts_with(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

/// Whether the resolution result whose call ends at `at` is propagated with
/// `?`, or mapped with `map_err` and then propagated or handed on whole
/// (`?`, or the end of the expression). `.map(..)` may come first.
fn is_propagated(chars: &[char], mut at: usize) -> bool {
    loop {
        at = skip_whitespace(chars, at);
        if chars.get(at) == Some(&'?') {
            return true;
        }
        if starts_with(chars, at, ".map(") {
            match close_paren(chars, at + 4) {
                Some(end) => at = end,
                None => return false,
            }
            continue;
        }
        if starts_with(chars, at, ".map_err(") {
            let Some(end) = close_paren(chars, at + 8) else {
                return false;
            };
            let next = skip_whitespace(chars, end);
            return matches!(chars.get(next), Some('?' | ';' | '}' | ')' | ','));
        }
        return false;
    }
}

/// Scenario: No code discards a state-root resolution error
#[test]
fn no_code_discards_a_state_root_resolution_error() -> TestResult {
    // @step Given the workspace's non-test Rust sources
    let sources = non_test_sources()?;

    // @step When the source-shape test scans every state-root resolution call
    let mut offenders = Vec::new();
    let mut calls = 0usize;
    for (relative, source) in &sources {
        if relative == Path::new(PATH_MODULE) {
            continue;
        }
        let code: Vec<char> = without_line_comments(source).chars().collect();
        let text: String = code.iter().collect();
        for call in RESOLUTION_CALLS {
            for (byte, _) in text.match_indices(call) {
                let start = text[..byte].chars().count();
                let before: String = code[start.saturating_sub(3)..start].iter().collect();
                if before == "fn " {
                    continue;
                }
                calls += 1;
                // `if let Err(error) = <call>` handles the error by name.
                let line_start = text[..byte].rfind('\n').map_or(0, |at| at + 1);
                let prefix = text[line_start..byte]
                    .trim_start()
                    .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == ':');
                if prefix.starts_with("if let Err(")
                    && !prefix.starts_with("if let Err(_")
                    && prefix.ends_with(") = ")
                {
                    continue;
                }
                let open = start + call.chars().count() - 1;
                let propagated =
                    close_paren(&code, open).is_some_and(|end| is_propagated(&code, end));
                if !propagated {
                    let line = text[..byte].matches('\n').count() + 1;
                    offenders.push(format!("{}:{line}: {call}", relative.display()));
                }
            }
        }
    }
    assert!(calls > 50, "found only {calls} resolution calls");

    // @step Then each result is propagated with ? or mapped with map_err, never discarded with ok, unwrap_or or a wildcard Err arm
    assert!(
        offenders.is_empty(),
        "{} state-root resolution results not propagated with ? or map_err:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
    Ok(())
}
