//! RPC-059 — Source-shape assertions for the loop-store LIFT.
//!
//! Feature: spec/features/rpc059-loop-store-lift.feature
//!
//! These tests pin the file layout for the lift of the loop_store
//! module out of `codelet/napi/src/scheduler/loop_store.rs` and into
//! `codelet/core/src/loops/mod.rs`. After the lift, the NAPI side
//! becomes a thin re-export shim and the lifted module has zero
//! `use napi` / `napi_derive` references.
//!
//! These tests are pure file scans — no compile dependency on the
//! lifted modules themselves — so they catch refactors that
//! accidentally re-introduce a NAPI dependency into the loop store.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/fspec-tui")
        .to_path_buf()
}

fn loops_core_dir() -> PathBuf {
    workspace_root().join("codelet/core/src/loops")
}

fn read_all_rust_files(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                out.push((path, content));
            }
        }
    }
    out
}

/// Scenario: Loop store module lives under codelet/core/src/loops/
#[test]
fn loops_module_lives_under_codelet_core() {
    // @step Given the directory codelet/core/src/loops/ exists
    let dir = loops_core_dir();
    assert!(
        dir.is_dir(),
        "codelet/core/src/loops/ directory should exist"
    );

    // @step Then it contains a file named "mod.rs"
    let mod_path = dir.join("mod.rs");
    assert!(
        mod_path.is_file(),
        "codelet/core/src/loops/mod.rs should exist"
    );
}

/// Scenario: codelet-core declares LoopEntry, LoopStore, and IdleCheckFn
#[test]
fn codelet_core_declares_loop_entry_store_idle_check() {
    // @step Given the file codelet/core/src/loops/mod.rs is compiled
    let path = loops_core_dir().join("mod.rs");
    let source = fs::read_to_string(&path).expect("read core/loops/mod.rs");

    // @step Then it declares a public struct named "LoopEntry"
    assert!(
        source.contains("pub struct LoopEntry"),
        "loops/mod.rs should declare pub struct LoopEntry"
    );

    // @step And LoopEntry has fields named id, session_id, prompt, interval_seconds, created_at, expires_at, last_run_at
    for field in [
        "pub id:",
        "pub session_id:",
        "pub prompt:",
        "pub interval_seconds:",
        "pub created_at:",
        "pub expires_at:",
        "pub last_run_at:",
    ] {
        assert!(
            source.contains(field),
            "LoopEntry should declare field {field:?}"
        );
    }

    // @step And it declares a public struct named "LoopStore"
    assert!(
        source.contains("pub struct LoopStore"),
        "loops/mod.rs should declare pub struct LoopStore"
    );

    // @step And it declares a public type alias named "IdleCheckFn"
    assert!(
        source.contains("pub type IdleCheckFn"),
        "loops/mod.rs should declare pub type IdleCheckFn"
    );
}

/// Scenario: LoopStore exposes the documented async API
#[test]
fn loop_store_exposes_documented_api() {
    // @step Given the file codelet/core/src/loops/mod.rs is compiled
    let path = loops_core_dir().join("mod.rs");
    let source = fs::read_to_string(&path).expect("read core/loops/mod.rs");

    // @step Then LoopStore declares a method named "instance" returning &'static LoopStore
    // @step And LoopStore declares a method named "cancel" taking &str and returning a future of bool
    // @step And LoopStore declares a method named "list_for_session" taking Uuid and returning a future of Vec<LoopEntry>
    // @step And LoopStore declares a method named "remove_for_session" taking Uuid and returning a future of usize
    // @step And LoopStore declares a method named "register_with_task_and_idle_check" taking LoopEntry plus on_fire and idle_check callbacks
    // @step And LoopStore declares a method named "try_register_with_task_and_idle_check" returning Result<(), String>
    // @step And LoopStore declares a method named "is_empty" returning a future of bool
    for method in [
        "pub fn instance(",
        "pub async fn cancel(",
        "pub async fn list_for_session(",
        "pub async fn remove_for_session(",
        "pub async fn register_with_task_and_idle_check(",
        "pub async fn try_register_with_task_and_idle_check(",
        "pub async fn is_empty(",
    ] {
        assert!(
            source.contains(method),
            "LoopStore should declare {method:?}"
        );
    }
}

/// Scenario: The lifted loop store has no NAPI references
#[test]
fn lifted_loop_store_has_no_napi_references() {
    // @step Given the directory codelet/core/src/loops/ exists
    let dir = loops_core_dir();
    assert!(dir.is_dir(), "loops core dir should exist");

    // @step Then no file under codelet/core/src/loops/ contains the text "use napi"
    // @step And no file under codelet/core/src/loops/ contains the text "napi_derive"
    for (path, content) in read_all_rust_files(&dir) {
        assert!(
            !content.contains("use napi"),
            "{} must not contain 'use napi' — the lift requires NAPI-free code",
            path.display()
        );
        assert!(
            !content.contains("napi_derive"),
            "{} must not contain 'napi_derive'",
            path.display()
        );
    }
}

/// Scenario: codelet/napi/src/scheduler/mod.rs re-exports the loops surface
#[test]
fn napi_scheduler_mod_reexports_loops_surface() {
    // @step Given the file codelet/napi/src/scheduler/mod.rs is compiled
    let path = workspace_root().join("codelet/napi/src/scheduler/mod.rs");
    let source = fs::read_to_string(&path).expect("read napi/src/scheduler/mod.rs");

    // @step Then it contains a re-export of codelet_core::loops::LoopStore
    // @step And it contains a re-export of codelet_core::loops::LoopEntry
    // @step And it contains a re-export of codelet_core::loops::IdleCheckFn
    assert!(
        source.contains("codelet_core::loops"),
        "napi/scheduler/mod.rs should reference codelet_core::loops"
    );
    for symbol in ["LoopStore", "LoopEntry", "IdleCheckFn"] {
        assert!(
            source.contains(symbol),
            "napi/scheduler/mod.rs should re-export {symbol}"
        );
    }
}

/// Scenario: codelet/napi/src/scheduler/loop_store.rs is deleted
#[test]
fn napi_loop_store_file_is_deleted() {
    // @step Given the directory codelet/napi/src/scheduler/ exists
    let dir = workspace_root().join("codelet/napi/src/scheduler");
    assert!(dir.is_dir(), "napi/src/scheduler/ should exist");

    // @step Then it does not contain a file named "loop_store.rs"
    let path = dir.join("loop_store.rs");
    assert!(
        !path.is_file(),
        "codelet/napi/src/scheduler/loop_store.rs should be deleted after the lift"
    );
}

/// Scenario: codelet-core lib.rs exports the loops module
#[test]
fn codelet_core_lib_exports_loops_module() {
    // @step Given the file codelet/core/src/lib.rs is compiled
    let path = workspace_root().join("codelet/core/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read core/src/lib.rs");

    // @step Then it declares a public module named "loops"
    assert!(
        source.contains("pub mod loops"),
        "codelet/core/src/lib.rs should declare pub mod loops"
    );
}
