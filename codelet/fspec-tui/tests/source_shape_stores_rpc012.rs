//! RPC-012 — Source-shape regression for the new `store/` module.
//!
//! Feature: spec/features/rpc012-source-shape.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::Path;

fn store_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src").join("store")
}

fn scan(dir: &Path) -> Vec<(std::path::PathBuf, String)> {
    let mut out = Vec::new();
    for path in common::collect_rs_files(dir) {
        let body = common::read_to_string_or_panic(&path);
        let code = common::strip_rust_comments(&body);
        out.push((path, code));
    }
    out
}

/// Scenario: Source-shape regression forbids Mutex/RwLock/atomics in store/
#[test]
fn store_module_has_no_mutex_rwlock_atomics_or_runtime_constructors() {
    // @step Given the directory codelet/fspec-tui/src/store/
    let dir = store_dir();
    assert!(dir.exists(), "expected store/ directory to exist");
    // @step When the test scans every .rs file under that directory
    let files = scan(&dir);
    assert!(!files.is_empty(), "expected at least one .rs file under store/");
    let forbidden = [
        // @step Then no file contains "std::sync::Mutex"
        "std::sync::Mutex",
        // @step And no file contains "tokio::sync::Mutex"
        "tokio::sync::Mutex",
        // @step And no file contains "std::sync::RwLock"
        "std::sync::RwLock",
        // @step And no file contains "tokio::sync::RwLock"
        "tokio::sync::RwLock",
        // @step And no file contains "AtomicUsize" or "AtomicBool" in a struct field type
        "AtomicUsize",
        "AtomicBool",
        // @step And no file contains "tokio::runtime::Builder"
        "tokio::runtime::Builder",
        // @step And no file contains "tokio::runtime::Runtime::new"
        "tokio::runtime::Runtime::new",
    ];
    let mut violations = Vec::new();
    for (path, code) in &files {
        for needle in forbidden {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "store/ module MUST be free of Mutex/RwLock/atomics/runtime-constructors. Violations: {violations:?}"
    );
}

/// Scenario: Source-shape regression forbids transport-layer imports in store/
#[test]
fn store_module_has_no_transport_layer_imports() {
    // @step Given the directory codelet/fspec-tui/src/store/
    let dir = store_dir();
    // @step When the test scans every .rs file under that directory
    let files = scan(&dir);
    let forbidden = [
        // @step Then no file contains the import "codelet_napi::"
        "codelet_napi::",
        // @step And no file contains the import "codelet_core::"
        "codelet_core::",
        // @step And no file contains the import "tarpc::"
        "tarpc::",
        // @step And no file contains the import "tokio_tungstenite::"
        "tokio_tungstenite::",
    ];
    let mut violations = Vec::new();
    for (path, code) in &files {
        for needle in forbidden {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "store/ module MUST NOT import transport-layer crates directly. Violations: {violations:?}"
    );
}

/// Scenario: Every file under codelet/fspec-tui/src/ is under 300 LoC
#[test]
fn new_rpc012_modules_are_each_under_300_loc() {
    // @step Given the directory codelet/fspec-tui/src/
    let src_dir = common::workspace_root().join("fspec-tui").join("src");
    // @step When the test counts the line-count of every .rs file under that directory
    fn count_lines(path: &Path) -> usize {
        let body = common::read_to_string_or_panic(path);
        body.lines().count()
    }
    let targets = [
        // @step Then store/board.rs has fewer than 300 lines
        ("store/board.rs", 300),
        // @step And store/agent_view.rs has fewer than 300 lines
        ("store/agent_view.rs", 300),
        // @step And store/mod.rs has fewer than 300 lines
        ("store/mod.rs", 300),
        // @step And views/navigator.rs has fewer than 300 lines
        ("views/navigator.rs", 300),
        // @step And views/board.rs has fewer than 300 lines
        ("views/board.rs", 300),
        // @step And views/agent.rs has fewer than 300 lines
        ("views/agent.rs", 300),
        // @step And app.rs has fewer than 300 lines
        // RPC-012 rule [10]: app.rs was split into app/mod.rs +
        // app/state.rs + app/bootstrap.rs + app/dispatch.rs +
        // app/events.rs so every child stays under 300 LoC.
        ("app/mod.rs", 300),
        ("app/state.rs", 300),
        ("app/bootstrap.rs", 300),
        ("app/dispatch.rs", 300),
        ("app/events.rs", 300),
    ];
    let mut violations = Vec::new();
    for (rel, ceiling) in targets {
        let path = src_dir.join(rel);
        assert!(path.exists(), "expected {} to exist", path.display());
        let lines = count_lines(&path);
        if lines >= ceiling {
            violations.push(format!("{rel}: {lines} lines >= {ceiling} ceiling"));
        }
    }
    // RPC-012 example [9]: legacy app.rs MUST NOT exist anymore — the
    // file was split into the app/ module above.
    assert!(
        !src_dir.join("app.rs").exists(),
        "codelet/fspec-tui/src/app.rs must not exist after RPC-012 split"
    );
    assert!(
        violations.is_empty(),
        "new RPC-012 modules MUST be under their LoC ceilings. Violations: {violations:?}"
    );
}
