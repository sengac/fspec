//! RPC-017 — Source-shape regression for the priority-reorder persistence port.
//!
//! Feature: spec/features/rpc017-source-shape.feature

use codelet_test_helpers::{OrFail, TestResult};
mod common;

fn src_dir() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    Ok(common::workspace_root()?.join("fspec-tui").join("src"))
}

fn workspace_root() -> Result<std::path::PathBuf, codelet_test_helpers::TestError> {
    common::workspace_root()
}

fn read_raw(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    if [
        "fspec-tui/src/components/mod.rs",
        "fspec-tui/src/transport/mod.rs",
        "fspec-tui/src/transport/embedded.rs",
        "fspec-tui/src/transport/websocket.rs",
        "fspec-tui/src/views/navigator_mux_events.rs",
        "rpc/src/lib.rs",
    ]
    .iter()
    .any(|entry| path.ends_with(entry))
    {
        return Ok(codelet_test_helpers::source_composition::read_source(path)
            .or_fail("resolve composed source entrypoint")?);
    }
    common::read_to_string_or_fail(path)
}

fn read_stripped(rel: &str) -> Result<String, codelet_test_helpers::TestError> {
    let path = src_dir()?.join(rel);
    let body = read_raw(&path)?;
    Ok(common::strip_rust_comments(&body))
}

fn count_lines_path(path: &std::path::Path) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(std::fs::read_to_string(path)
        .or_fail("read physical source for line count")?
        .lines()
        .count())
}

include!("source_shape_rpc017/shared_helpers.rs");

/// Scenario: FspecService trait gains the move_work_unit_up / _down RPC methods
#[test]
fn fspec_service_trait_gains_the_move_work_unit_up_and_down_rpc_methods() -> TestResult {
    // @step Given rust/rpc/src/lib.rs after RPC-017 lands
    let rpc_lib = workspace_root()?.join("rpc").join("src").join("lib.rs");
    let body = read_raw(&rpc_lib)?;

    // @step Then the file contains the substring "async fn move_work_unit_up(workspace: String, id: String)"
    assert!(
        body.contains("async fn move_work_unit_up(workspace: String, id: String)"),
        "FspecService must declare `async fn move_work_unit_up(workspace: String, id: String)`"
    );

    // @step And the file contains the substring "async fn move_work_unit_down(workspace: String, id: String)"
    assert!(
        body.contains("async fn move_work_unit_down(workspace: String, id: String)"),
        "FspecService must declare `async fn move_work_unit_down(workspace: String, id: String)`"
    );

    // @step And the FspecServiceImpl body contains the substring "codelet_core::work_units_write::move_work_unit"
    assert!(
        body.contains("codelet_core::work_units_write::move_work_unit"),
        "FspecServiceImpl must delegate to codelet_core::work_units_write::move_work_unit"
    );
    Ok(())
}

/// Scenario: FspecBackend trait gains the move_work_unit_up / _down methods
#[test]
fn fspec_backend_trait_gains_the_move_work_unit_up_and_down_methods() -> TestResult {
    // @step Given rust/fspec-tui/src/transport/mod.rs after RPC-017 lands
    let path = src_dir()?.join("transport").join("mod.rs");
    let body = read_raw(&path)?;

    // @step Then the file contains the substring "async fn move_work_unit_up"
    assert!(
        body.contains("async fn move_work_unit_up"),
        "transport/mod.rs must declare `async fn move_work_unit_up`"
    );

    // @step And the file contains the substring "async fn move_work_unit_down"
    assert!(
        body.contains("async fn move_work_unit_down"),
        "transport/mod.rs must declare `async fn move_work_unit_down`"
    );

    // @step And both methods take `workspace: String, id: String` and return `Result<()>`
    assert!(
        body.contains("move_work_unit_up(&self, workspace: String, id: String) -> Result<()>"),
        "move_work_unit_up signature must be (&self, id: String) -> Result<()>"
    );
    assert!(
        body.contains("move_work_unit_down(&self, workspace: String, id: String) -> Result<()>"),
        "move_work_unit_down signature must be (&self, id: String) -> Result<()>"
    );
    Ok(())
}

/// Scenario: Both transports implement the new FspecBackend methods
#[test]
fn both_transports_implement_the_new_fspec_backend_methods() -> TestResult {
    // @step Given the rust/fspec-tui crate after RPC-017 lands
    let embedded = src_dir()?.join("transport").join("embedded.rs");
    let websocket = src_dir()?.join("transport").join("websocket.rs");

    // @step Then rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_up"
    let emb_body = read_raw(&embedded)?;
    assert!(
        emb_body.contains("async fn move_work_unit_up"),
        "embedded.rs must implement async fn move_work_unit_up"
    );

    // @step And rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_down"
    assert!(
        emb_body.contains("async fn move_work_unit_down"),
        "embedded.rs must implement async fn move_work_unit_down"
    );

    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_up"
    let ws_body = read_raw(&websocket)?;
    assert!(
        ws_body.contains("async fn move_work_unit_up"),
        "websocket.rs must implement async fn move_work_unit_up"
    );

    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_down"
    assert!(
        ws_body.contains("async fn move_work_unit_down"),
        "websocket.rs must implement async fn move_work_unit_down"
    );
    Ok(())
}

/// Scenario: Action::ReorderUp / ReorderDown handlers are no longer no-ops
#[test]
fn action_reorder_handlers_are_no_longer_no_ops() -> TestResult {
    // @step Given rust/fspec-tui/src/app/dispatch.rs and its reorder helper rust/fspec-tui/src/app/dispatch_reorder.rs after RPC-450 lands
    let dispatch = src_dir()?.join("app").join("dispatch.rs");
    let reorder = src_dir()?.join("app").join("dispatch_reorder.rs");
    let body = format!("{}\n{}", read_raw(&dispatch)?, read_raw(&reorder)?);
    // BUG-459: rustfmt splits `backend` / `.move_work_unit_up(` across lines.
    let stripped: String = common::strip_rust_comments(&body)
        .split_whitespace()
        .collect();

    // @step Then the file contains the substring "backend.move_work_unit_up"
    assert!(
        stripped.contains("backend.move_work_unit_up"),
        "dispatch.rs must call backend.move_work_unit_up"
    );

    // @step And the file contains the substring "backend.move_work_unit_down"
    assert!(
        stripped.contains("backend.move_work_unit_down"),
        "dispatch.rs must call backend.move_work_unit_down"
    );

    // @step And the file does NOT contain the substring "RPC-012 architecture note [1]: persistence is out of scope"
    assert!(
        !body.contains("RPC-012 architecture note [1]: persistence is out of scope"),
        "dispatch.rs must no longer carry the RPC-012 placeholder comment"
    );
    Ok(())
}

/// Scenario: Views do not directly import codelet_core / tarpc
#[test]
fn views_do_not_directly_import_codelet_core_or_tarpc() -> TestResult {
    // @step Given the directory rust/fspec-tui/src/views/ after RPC-017 lands
    let views_dir = src_dir()?.join("views");

    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");

    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_fail(path)?;
        let code = common::strip_rust_comments(&body);
        // @step Then no file imports `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
        for needle in ["codelet_core::", "tarpc::", "tokio_tungstenite::"] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
        // @step And no file constructs `tokio::runtime::Builder` or `Runtime::new()`
        for needle in ["tokio::runtime::Builder", "Runtime::new()"] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-017 must preserve view-layer isolation. Violations: {violations:?}"
    );
    Ok(())
}

// Allow `read_stripped` to remain importable for future tests without
// triggering dead_code if not used here.
#[allow(dead_code)]
fn _ensure_helpers_in_scope() -> Result<(), codelet_test_helpers::TestError> {
    let _ = read_stripped("lib.rs")?;
    Ok(())
}
