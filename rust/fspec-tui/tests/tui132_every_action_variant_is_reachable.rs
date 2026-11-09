//! TUI-132: every Action variant is reachable from the UI.
//!
//! Feature: spec/features/every-tui-action-is-reachable-from-the-ui.feature
//!
//! A pure source scan (no runtime, no clock). Each `Action` variant
//! declared in `src/components/action_*.rs` must be constructed somewhere
//! in the production source (an emitter) and matched by a production
//! handler. Test files, `#[cfg(test)]` items, `*for_test*` / `test_*`
//! functions and comments are stripped before the scan.
//!
//! An occurrence of `Action::<Variant>` is a handler (a pattern) when the
//! next token after its payload (and any closing parens) is `=>`, `|`,
//! `if` or a single `=`, or when it sits inside `matches!(..)`; otherwise
//! it is an emitter (a construction).

use codelet_test_helpers::{OrFail, TestResult};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every variant declared in components/action_*.rs (12-space indented
/// capitalised identifiers inside the compose_action_* macros).
fn declared_variants() -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let components = src_dir().join("components");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&components)
        .or_fail("read src/components")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("action_") && n.ends_with(".rs"))
        })
        .collect();
    files.sort();
    let mut variants = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).or_fail("read an action_*.rs file")?;
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("            ") else {
                continue;
            };
            if !rest.starts_with(|c: char| c.is_ascii_uppercase()) {
                continue;
            }
            let name: String = rest
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            variants.push(name);
        }
    }
    Ok(variants)
}

fn is_test_file(path: &Path) -> bool {
    if path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("_tests.rs")
        || name == "tests.rs"
        || name.starts_with("tests_")
        || name.contains("_tests_")
        || name == "test_support.rs"
        || name.ends_with("_test_support.rs")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(dir).or_fail("read a source directory")? {
        let path = entry.or_fail("read a directory entry")?.path();
        if path.is_dir() {
            collect_rs(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Blank `//` and `/* */` comments, keeping byte offsets and newlines.
fn strip_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_str = true;
            i += 1;
            continue;
        }
        if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                if bytes[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            let end = (i + 2).min(bytes.len());
            for byte in out.iter_mut().take(end).skip(i) {
                *byte = b' ';
            }
            i = end;
            continue;
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Blank the brace-balanced item that starts at or after `from`
/// (through its closing `}`), or through the `;` of a bodiless item.
fn blank_item(text: &mut [u8], start: usize, from: usize) {
    let mut j = from;
    while j < text.len() && text[j] != b'{' && text[j] != b';' {
        j += 1;
    }
    if j >= text.len() {
        return;
    }
    let mut end = j;
    if text[j] == b'{' {
        let mut depth = 0usize;
        while end < text.len() {
            match text[end] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }
    }
    let end = (end + 1).min(text.len());
    for byte in text.iter_mut().take(end).skip(start) {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

/// Blank `#[cfg(test)]` items and `fn *for_test*` / `fn test_*` bodies.
fn strip_test_items(src: &str) -> String {
    let mut text = src.as_bytes().to_vec();
    let marker = "#[cfg(test)]";
    let mut starts: Vec<usize> = src.match_indices(marker).map(|(i, _)| i).collect();
    starts.reverse();
    for start in starts {
        blank_item(&mut text, start, start + marker.len());
    }
    let snapshot = String::from_utf8_lossy(&text).into_owned();
    let mut fns: Vec<usize> = Vec::new();
    for (i, _) in snapshot.match_indices("fn ") {
        let name: String = snapshot[i + 3..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.contains("for_test") || name.starts_with("test_") {
            fns.push(i);
        }
    }
    fns.reverse();
    for start in fns {
        blank_item(&mut text, start, start);
    }
    String::from_utf8_lossy(&text).into_owned()
}

fn production_sources() -> Result<Vec<(PathBuf, String)>, codelet_test_helpers::TestError> {
    let mut files = Vec::new();
    collect_rs(&src_dir(), &mut files)?;
    files.sort();
    let mut out = Vec::new();
    for path in files {
        if is_test_file(&path) {
            continue;
        }
        let raw = std::fs::read_to_string(&path).or_fail("read a source file")?;
        out.push((path, strip_test_items(&strip_comments(&raw))));
    }
    Ok(out)
}

/// Skip one balanced `(..)` or `{..}` group starting at `i` (after spaces).
fn skip_group(text: &[u8], mut i: usize) -> usize {
    while i < text.len() && text[i].is_ascii_whitespace() {
        i += 1;
    }
    let (open, close) = match text.get(i) {
        Some(b'(') => (b'(', b')'),
        Some(b'{') => (b'{', b'}'),
        _ => return i,
    };
    let mut depth = 0usize;
    while i < text.len() {
        if text[i] == open {
            depth += 1;
        } else if text[i] == close {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    i
}

/// True when the occurrence at `start..end` is a pattern (a handler).
fn is_pattern(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    let mut i = skip_group(bytes, end);
    while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b')') {
        i += 1;
    }
    let next = &text[i..(i + 3).min(text.len())];
    if next.starts_with("=>")
        || next.starts_with('|')
        || (next.starts_with("if") && !next[2..].starts_with(|c: char| c.is_alphanumeric()))
        || (next.starts_with('=') && !next.starts_with("=="))
    {
        return true;
    }
    // Inside an unclosed `matches!(` that opens before the occurrence.
    let window_start = start.saturating_sub(400);
    let before = &text[window_start..start];
    if let Some(k) = before.rfind("matches!(") {
        let mut depth = 0i32;
        for c in before[k + "matches!".len()..].chars() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        return true;
    }
    false
}

#[derive(Default)]
struct Uses {
    emitters: Vec<String>,
    handlers: Vec<String>,
}

fn scan() -> Result<BTreeMap<String, Uses>, codelet_test_helpers::TestError> {
    let variants = declared_variants()?;
    let sources = production_sources()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut uses: BTreeMap<String, Uses> = BTreeMap::new();
    for variant in variants {
        let needle = format!("Action::{variant}");
        let entry = uses.entry(variant.clone()).or_default();
        for (path, text) in &sources {
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .display()
                .to_string();
            for (start, _) in text.match_indices(&needle) {
                let end = start + needle.len();
                let boundary = text[end..]
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                if !boundary {
                    continue;
                }
                let line = text[..start].matches('\n').count() + 1;
                let at = format!("{rel}:{line}");
                if is_pattern(text, start, end) {
                    entry.handlers.push(at);
                } else {
                    entry.emitters.push(at);
                }
            }
        }
    }
    Ok(uses)
}

/// Scenario: Every Action variant has a production emitter and a production handler
#[test]
fn every_action_variant_has_a_production_emitter_and_a_production_handler() -> TestResult {
    // @step Given the Action variants declared in components/action_*.rs
    let uses = scan()?;
    assert!(
        uses.len() > 200,
        "expected the full Action enum, found {}",
        uses.len()
    );

    // @step When the TUI source outside tests is scanned for each variant
    let mut no_emitter: Vec<&str> = Vec::new();
    let mut no_handler: Vec<&str> = Vec::new();
    for (variant, found) in &uses {
        if found.emitters.is_empty() {
            no_emitter.push(variant);
        }
        if found.handlers.is_empty() {
            no_handler.push(variant);
        }
    }

    // @step Then every variant is constructed somewhere outside tests
    // @step And every variant is matched by a handler outside tests
    // @step And there is no exception
    assert!(
        no_emitter.is_empty(),
        "Action variants with no production emitter (wire them up or delete them): {no_emitter:?}"
    );
    assert!(
        no_handler.is_empty(),
        "Action variants with no production handler (wire them up or delete them): {no_handler:?}"
    );
    Ok(())
}

/// Scenario: The scaffolding nothing shows is gone
#[test]
fn the_scaffolding_nothing_shows_is_gone() -> TestResult {
    // @step Given the TUI source outside tests
    let sources = production_sources()?;
    let all: String = sources.iter().map(|(_, text)| text.as_str()).collect();

    // @step Then it declares no MouseTrackingToggle, no DisconnectDialog and no StatusDialog
    // @step And no Action::ReEnableMouseTracking, Action::ManualReconnect or Action::RequestDeleteSession
    // @step And no request_manual_reconnect and no try_dispatch_slash_debug
    for gone in [
        "MouseTrackingToggle",
        "DisconnectDialog",
        "DISCONNECT_DIALOG_ID",
        "StatusDialog",
        "ReEnableMouseTracking",
        "ManualReconnect",
        "RequestDeleteSession",
        "request_manual_reconnect",
        "try_dispatch_slash_debug",
    ] {
        assert!(!all.contains(gone), "the TUI source still has {gone}");
    }
    for file in [
        "mouse/toggle.rs",
        "components/disconnect_dialog.rs",
        "components/status_dialog.rs",
    ] {
        assert!(!src_dir().join(file).exists(), "src/{file} still exists");
    }
    Ok(())
}
