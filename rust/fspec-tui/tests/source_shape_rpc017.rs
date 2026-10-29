//! RPC-017 — Source-shape regression for the priority-reorder persistence port.
//!
//! Feature: spec/features/rpc017-source-shape.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn workspace_root() -> std::path::PathBuf {
    common::workspace_root()
}

fn read_raw(path: &std::path::Path) -> String {
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
        return codelet_test_helpers::source_composition::read_source(path)
            .expect("resolve composed source entrypoint");
    }
    common::read_to_string_or_panic(path)
}

fn read_stripped(rel: &str) -> String {
    let path = src_dir().join(rel);
    let body = read_raw(&path);
    common::strip_rust_comments(&body)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    std::fs::read_to_string(path)
        .expect("read physical source for line count")
        .lines()
        .count()
}

include!("source_shape_rpc017/shared_helpers.rs");

/// Scenario: FspecService trait gains the move_work_unit_up / _down RPC methods
#[test]
fn fspec_service_trait_gains_the_move_work_unit_up_and_down_rpc_methods() {
    // @step Given rust/rpc/src/lib.rs after RPC-017 lands
    let rpc_lib = workspace_root().join("rpc").join("src").join("lib.rs");
    let body = read_raw(&rpc_lib);

    // @step Then the file contains the substring "async fn move_work_unit_up(id: String)"
    assert!(
        body.contains("async fn move_work_unit_up(id: String)"),
        "FspecService must declare `async fn move_work_unit_up(id: String)`"
    );

    // @step And the file contains the substring "async fn move_work_unit_down(id: String)"
    assert!(
        body.contains("async fn move_work_unit_down(id: String)"),
        "FspecService must declare `async fn move_work_unit_down(id: String)`"
    );

    // @step And the FspecServiceImpl body contains the substring "codelet_core::work_units_write::move_work_unit"
    assert!(
        body.contains("codelet_core::work_units_write::move_work_unit"),
        "FspecServiceImpl must delegate to codelet_core::work_units_write::move_work_unit"
    );
}

/// Scenario: FspecBackend trait gains the move_work_unit_up / _down methods
#[test]
fn fspec_backend_trait_gains_the_move_work_unit_up_and_down_methods() {
    // @step Given rust/fspec-tui/src/transport/mod.rs after RPC-017 lands
    let path = src_dir().join("transport").join("mod.rs");
    let body = read_raw(&path);

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

    // @step And both methods take `id: String` and return `Result<()>`
    assert!(
        body.contains("move_work_unit_up(&self, id: String) -> Result<()>"),
        "move_work_unit_up signature must be (&self, id: String) -> Result<()>"
    );
    assert!(
        body.contains("move_work_unit_down(&self, id: String) -> Result<()>"),
        "move_work_unit_down signature must be (&self, id: String) -> Result<()>"
    );
}

/// Scenario: Both transports implement the new FspecBackend methods
#[test]
fn both_transports_implement_the_new_fspec_backend_methods() {
    // @step Given the rust/fspec-tui crate after RPC-017 lands
    let embedded = src_dir().join("transport").join("embedded.rs");
    let websocket = src_dir().join("transport").join("websocket.rs");

    // @step Then rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_up"
    let emb_body = read_raw(&embedded);
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
    let ws_body = read_raw(&websocket);
    assert!(
        ws_body.contains("async fn move_work_unit_up"),
        "websocket.rs must implement async fn move_work_unit_up"
    );

    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_down"
    assert!(
        ws_body.contains("async fn move_work_unit_down"),
        "websocket.rs must implement async fn move_work_unit_down"
    );
}

/// Scenario: Action::ReorderUp / ReorderDown handlers are no longer no-ops
#[test]
fn action_reorder_handlers_are_no_longer_no_ops() {
    // @step Given rust/fspec-tui/src/app/dispatch.rs after RPC-017 lands
    let dispatch = src_dir().join("app").join("dispatch.rs");
    let body = read_raw(&dispatch);
    let stripped = common::strip_rust_comments(&body);

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
}

/// Scenario: NAPI exports for move_work_unit_up / _down delegate to the shared helper
#[test]
fn napi_exports_delegate_to_the_shared_helper() {
    // @step Given rust/napi/src/work_units_watcher.rs after RPC-017 lands
    let path = workspace_root()
        .join("napi")
        .join("src")
        .join("work_units_watcher.rs");
    let body = read_raw(&path);

    // @step Then the file contains the substring "pub fn move_work_unit_up"
    assert!(
        body.contains("pub fn move_work_unit_up"),
        "napi work_units_watcher.rs must export pub fn move_work_unit_up"
    );

    // @step And the file contains the substring "pub fn move_work_unit_down"
    assert!(
        body.contains("pub fn move_work_unit_down"),
        "napi work_units_watcher.rs must export pub fn move_work_unit_down"
    );

    // @step And both function bodies contain the substring "codelet_core::work_units_write::move_work_unit"
    assert!(
        body.contains("codelet_core::work_units_write::move_work_unit"),
        "napi reorder exports must delegate to codelet_core::work_units_write::move_work_unit"
    );
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc
#[test]
fn views_do_not_directly_import_codelet_core_napi_tarpc() {
    // @step Given the directory rust/fspec-tui/src/views/ after RPC-017 lands
    let views_dir = src_dir().join("views");

    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");

    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        // @step Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
        for needle in [
            "codelet_napi::",
            "codelet_core::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
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
}

// Allow `read_stripped` to remain importable for future tests without
// triggering dead_code if not used here.
#[allow(dead_code)]
fn _ensure_helpers_in_scope() {
    let _ = read_stripped("lib.rs");
}
