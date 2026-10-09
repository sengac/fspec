//! RPC-050 — Source-shape regression tests for work-unit binding +
//! /detach.
//!
//! Feature: spec/features/slash-command-detach-source-shape.feature
//!
//! Pins the file layout invariants for the RPC-050 wiring:
//!   * No file under `rust/fspec-tui/src/` matches "codelet_napi"
//!     (post-RPC-002 invariant).
//!   * Every file under `rust/fspec-tui/src/app/`,
//!     `rust/fspec-tui/src/views/agent/`, and
//!     `rust/fspec-tui/src/store/agent_view/` is strictly less than
//!     300 lines of code.
//!   * `rust/fspec-tui/src/app/dispatch.rs` is strictly less than
//!     300 lines of code.
//!   * `rust/fspec-tui/src/app/dispatch_slash_commands.rs` is strictly less
//!     than 300 lines of code.
//!   * `components::Action` declares the RPC-050 attach variants.
//!   * `dispatch_work_unit_binding.rs` declares the RPC-050 attach helpers.
//!
//! BUG-205: the `/detach` command was removed, so the invariants now pin
//! the ABSENCE of `WorkUnitDetached(` and `handle_slash_detach` /
//! `handle_work_unit_detached` from the source.

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
    // @step Given the rust/fspec-tui/src/ tree after the RPC-050 changes
    let src = fspec_tui_src();
    let files = collect_rs_files(&src);

    // @step Then no file under rust/fspec-tui/src/ matches "codelet_napi"
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

    // @step And every file under rust/fspec-tui/src/app/, rust/fspec-tui/src/views/agent/, and rust/fspec-tui/src/store/agent_view/ is strictly less than 300 lines of code
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

    // @step And rust/fspec-tui/src/app/dispatch.rs is strictly less than 300 lines of code
    let dispatch = src.join("app").join("dispatch.rs");
    let n = count_lines(&dispatch);
    assert!(n < 300, "app/dispatch.rs has {n} lines (must be < 300)");

    // @step And rust/fspec-tui/src/app/dispatch_slash_commands.rs is strictly less than 300 lines of code
    let dispatch020 = src.join("app").join("dispatch_slash_commands.rs");
    let n = count_lines(&dispatch020);
    assert!(
        n < 300,
        "app/dispatch_slash_commands.rs has {n} lines (must be < 300)"
    );
}

/// Scenario: components::Action declares the new RPC-050 variants
#[test]
fn action_enum_declares_rpc050_variants() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-050 lands
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

    // @step And (BUG-205) the file declares NO "WorkUnitDetached(" Action variant
    assert!(
        !body.contains("WorkUnitDetached("),
        "components/mod.rs must NOT declare Action::WorkUnitDetached after BUG-205 removed /detach",
    );
}

/// Scenario: dispatch_work_unit_binding.rs declares the RPC-050 attach
/// helpers (BUG-205: the detach helpers are gone)
#[test]
fn dispatch_work_unit_binding_declares_attach_helpers_and_no_detach_helpers() {
    // @step Given rust/fspec-tui/src/app/dispatch_work_unit_binding.rs after RPC-050 lands
    let path = fspec_tui_src().join("app").join("dispatch_work_unit_binding.rs");
    let body = read_raw(&path);

    // @step Then the file declares "handle_attach_work_unit_to_session"
    assert!(
        body.contains("handle_attach_work_unit_to_session"),
        "must declare the attach entry helper",
    );
    // @step And the file declares "handle_work_unit_attached"
    assert!(
        body.contains("handle_work_unit_attached"),
        "must declare the attach Ok-branch fold helper",
    );
    // @step And (BUG-205) the file declares NO "handle_slash_detach"
    assert!(
        !body.contains("handle_slash_detach"),
        "BUG-205: handle_slash_detach must be removed with the /detach command",
    );
    // @step And (BUG-205) the file declares NO "handle_work_unit_detached"
    assert!(
        !body.contains("handle_work_unit_detached"),
        "BUG-205: handle_work_unit_detached must be removed with the /detach command",
    );
}
