//! RPC-050 — Source-shape regression tests for work-unit binding +
//! /detach.
//!
//! Feature: spec/features/slash-command-detach-source-shape.feature
//!
//! Pins the file layout invariants for the new RPC-050 wiring:
//!   * No file under `codelet/fspec-tui/src/` matches "codelet_napi"
//!     (post-RPC-002 invariant).
//!   * Every file under `codelet/fspec-tui/src/app/`,
//!     `codelet/fspec-tui/src/views/agent/`, and
//!     `codelet/fspec-tui/src/store/agent_view/` is strictly less than
//!     300 lines of code.
//!   * `codelet/fspec-tui/src/app/dispatch.rs` is strictly less than
//!     300 lines of code.
//!   * `codelet/fspec-tui/src/app/dispatch_rpc020.rs` is strictly less
//!     than 300 lines of code.
//!   * `components::Action` declares the three new RPC-050 variants.
//!   * `dispatch_rpc050.rs` declares the four RPC-050 helpers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::{Path, PathBuf};

fn fspec_tui_src() -> PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

fn read_raw(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read file")
}

fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn count_lines(path: &Path) -> usize {
    read_raw(path).lines().count()
}

/// Scenario: No codelet_napi reference and the 300-LoC ceiling holds
#[test]
fn no_codelet_napi_reference_and_300_loc_ceiling() {
    // @step Given the codelet/fspec-tui/src/ tree after the RPC-050 changes
    let src = fspec_tui_src();
    let files = collect_rs_files(&src);

    // @step Then no file under codelet/fspec-tui/src/ matches "codelet_napi"
    let mut napi_violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path);
        let code = strip_rust_comments(&body);
        if code.contains("codelet_napi") {
            napi_violations.push(format!("{} contains codelet_napi", path.display()));
        }
    }
    assert!(
        napi_violations.is_empty(),
        "codelet_napi references detected in fspec-tui/src/: {napi_violations:?}",
    );

    // @step And every file under codelet/fspec-tui/src/app/, codelet/fspec-tui/src/views/agent/, and codelet/fspec-tui/src/store/agent_view/ is strictly less than 300 lines of code
    let ceiling_dirs = [
        src.join("app"),
        src.join("views").join("agent"),
        src.join("store").join("agent_view"),
    ];
    let mut loc_violations: Vec<String> = Vec::new();
    for dir in &ceiling_dirs {
        for path in collect_rs_files(dir) {
            let n = count_lines(&path);
            if n >= 300 {
                loc_violations.push(format!("{} has {n} lines (must be < 300)", path.display()));
            }
        }
    }
    assert!(
        loc_violations.is_empty(),
        "300-LoC ceiling violations: {loc_violations:?}",
    );

    // @step And codelet/fspec-tui/src/app/dispatch.rs is strictly less than 300 lines of code
    let dispatch = src.join("app").join("dispatch.rs");
    let n = count_lines(&dispatch);
    assert!(n < 300, "app/dispatch.rs has {n} lines (must be < 300)");

    // @step And codelet/fspec-tui/src/app/dispatch_rpc020.rs is strictly less than 300 lines of code
    let dispatch020 = src.join("app").join("dispatch_rpc020.rs");
    let n = count_lines(&dispatch020);
    assert!(
        n < 300,
        "app/dispatch_rpc020.rs has {n} lines (must be < 300)"
    );
}

/// Scenario: components::Action declares the new RPC-050 variants
#[test]
fn action_enum_declares_rpc050_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-050 lands
    let path = fspec_tui_src().join("components").join("mod.rs");
    let body = read_raw(&path);

    // @step Then the file declares "AttachWorkUnitToSession(" as an Action variant
    assert!(
        body.contains("AttachWorkUnitToSession("),
        "components/mod.rs must declare Action::AttachWorkUnitToSession(String) variant",
    );

    // @step And the file declares "WorkUnitAttached(" as an Action variant
    assert!(
        body.contains("WorkUnitAttached("),
        "components/mod.rs must declare Action::WorkUnitAttached(SessionId, WorkUnitContext) variant",
    );

    // @step And the file declares "WorkUnitDetached(" as an Action variant
    assert!(
        body.contains("WorkUnitDetached("),
        "components/mod.rs must declare Action::WorkUnitDetached(SessionId) variant",
    );
}

/// Scenario: dispatch_rpc050.rs declares the new RPC-050 helpers
#[test]
fn dispatch_rpc050_declares_helpers() {
    // @step Given codelet/fspec-tui/src/app/dispatch_rpc050.rs after RPC-050 lands
    let path = fspec_tui_src().join("app").join("dispatch_rpc050.rs");
    let body = read_raw(&path);

    // @step Then the file declares "handle_attach_work_unit_to_session"
    assert!(
        body.contains("handle_attach_work_unit_to_session"),
        "dispatch_rpc050.rs must declare handle_attach_work_unit_to_session helper",
    );

    // @step And the file declares "handle_work_unit_attached"
    assert!(
        body.contains("handle_work_unit_attached"),
        "dispatch_rpc050.rs must declare handle_work_unit_attached helper",
    );

    // @step And the file declares "handle_work_unit_detached"
    assert!(
        body.contains("handle_work_unit_detached"),
        "dispatch_rpc050.rs must declare handle_work_unit_detached helper",
    );

    // @step And the file declares "handle_slash_detach"
    assert!(
        body.contains("handle_slash_detach"),
        "dispatch_rpc050.rs must declare handle_slash_detach helper",
    );
}
