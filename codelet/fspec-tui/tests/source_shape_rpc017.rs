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
    common::read_to_string_or_panic(path)
}

fn read_stripped(rel: &str) -> String {
    let path = src_dir().join(rel);
    let body = common::read_to_string_or_panic(&path);
    common::strip_rust_comments(&body)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    common::read_to_string_or_panic(path).lines().count()
}

/// Scenario: codelet_common::file_lock is lifted out of schedule_handler.rs
#[test]
fn codelet_common_file_lock_is_lifted_out_of_schedule_handler() {
    // @step Given the codelet workspace after RPC-017 lands
    let file_lock = workspace_root().join("common").join("src").join("file_lock.rs");
    let common_lib = workspace_root().join("common").join("src").join("lib.rs");
    let schedule_handler = workspace_root()
        .join("napi")
        .join("src")
        .join("schedule_handler.rs");

    // @step Then the file codelet/common/src/file_lock.rs exists
    assert!(
        file_lock.exists(),
        "codelet/common/src/file_lock.rs must exist after RPC-017"
    );

    // @step And codelet/common/src/file_lock.rs contains the substring "pub fn with_file_lock"
    let file_lock_body = read_raw(&file_lock);
    assert!(
        file_lock_body.contains("pub fn with_file_lock"),
        "codelet/common/src/file_lock.rs must export `pub fn with_file_lock`"
    );

    // @step And codelet/common/src/lib.rs contains the substring "pub mod file_lock"
    let lib_body = read_raw(&common_lib);
    assert!(
        lib_body.contains("pub mod file_lock"),
        "codelet/common/src/lib.rs must declare `pub mod file_lock`"
    );

    // @step And codelet/napi/src/schedule_handler.rs contains the substring "codelet_common::file_lock"
    let sched_body = read_raw(&schedule_handler);
    let sched_stripped = common::strip_rust_comments(&sched_body);
    assert!(
        sched_stripped.contains("codelet_common::file_lock"),
        "schedule_handler.rs must depend on codelet_common::file_lock"
    );

    // @step And codelet/napi/src/schedule_handler.rs does NOT contain the substring "fn acquire_lock"
    assert!(
        !sched_stripped.contains("fn acquire_lock"),
        "schedule_handler.rs must no longer inline `fn acquire_lock`"
    );

    // @step And codelet/napi/src/schedule_handler.rs does NOT contain the substring "fn release_lock"
    assert!(
        !sched_stripped.contains("fn release_lock"),
        "schedule_handler.rs must no longer inline `fn release_lock`"
    );
}

/// Scenario: codelet_core::work_units_write module exists and stays under 300 LoC
#[test]
fn codelet_core_work_units_write_module_exists_and_stays_under_300_loc() {
    // @step Given the codelet workspace after RPC-017 lands
    let wuw = workspace_root().join("core").join("src").join("work_units_write.rs");
    let core_lib = workspace_root().join("core").join("src").join("lib.rs");

    // @step Then the file codelet/core/src/work_units_write.rs exists
    assert!(
        wuw.exists(),
        "codelet/core/src/work_units_write.rs must exist after RPC-017"
    );

    // @step And codelet/core/src/work_units_write.rs has fewer than 300 lines
    let lines = count_lines_path(&wuw);
    assert!(
        lines < 300,
        "codelet/core/src/work_units_write.rs must stay < 300 LoC (currently {lines})"
    );

    let body = read_raw(&wuw);

    // @step And codelet/core/src/work_units_write.rs contains the substring "pub fn move_work_unit"
    assert!(
        body.contains("pub fn move_work_unit"),
        "work_units_write.rs must export `pub fn move_work_unit`"
    );

    // @step And codelet/core/src/work_units_write.rs contains the substring "pub enum Direction"
    assert!(
        body.contains("pub enum Direction"),
        "work_units_write.rs must export `pub enum Direction`"
    );

    // @step And codelet/core/src/lib.rs contains the substring "pub mod work_units_write"
    let core_lib_body = read_raw(&core_lib);
    assert!(
        core_lib_body.contains("pub mod work_units_write"),
        "codelet/core/src/lib.rs must declare `pub mod work_units_write`"
    );

    // @step And codelet/core/src/work_units.rs (the read-side module) still exports `pub fn read_snapshot` and `pub struct WorkUnitsWatcher`
    let work_units_rs = workspace_root().join("core").join("src").join("work_units.rs");
    let work_units_body = read_raw(&work_units_rs);
    assert!(
        work_units_body.contains("pub fn read_snapshot"),
        "work_units.rs (read-side) must still export read_snapshot"
    );
    assert!(
        work_units_body.contains("pub struct WorkUnitsWatcher"),
        "work_units.rs (read-side) must still export WorkUnitsWatcher"
    );
}

/// Scenario: FspecService trait gains the move_work_unit_up / _down RPC methods
#[test]
fn fspec_service_trait_gains_the_move_work_unit_up_and_down_rpc_methods() {
    // @step Given codelet/rpc/src/lib.rs after RPC-017 lands
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
    // @step Given codelet/fspec-tui/src/transport/mod.rs after RPC-017 lands
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
    // @step Given the codelet/fspec-tui crate after RPC-017 lands
    let embedded = src_dir().join("transport").join("embedded.rs");
    let websocket = src_dir().join("transport").join("websocket.rs");

    // @step Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_up"
    let emb_body = read_raw(&embedded);
    assert!(
        emb_body.contains("async fn move_work_unit_up"),
        "embedded.rs must implement async fn move_work_unit_up"
    );

    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn move_work_unit_down"
    assert!(
        emb_body.contains("async fn move_work_unit_down"),
        "embedded.rs must implement async fn move_work_unit_down"
    );

    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_up"
    let ws_body = read_raw(&websocket);
    assert!(
        ws_body.contains("async fn move_work_unit_up"),
        "websocket.rs must implement async fn move_work_unit_up"
    );

    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn move_work_unit_down"
    assert!(
        ws_body.contains("async fn move_work_unit_down"),
        "websocket.rs must implement async fn move_work_unit_down"
    );
}

/// Scenario: Action::ReorderUp / ReorderDown handlers are no longer no-ops
#[test]
fn action_reorder_handlers_are_no_longer_no_ops() {
    // @step Given codelet/fspec-tui/src/app/dispatch.rs after RPC-017 lands
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
    // @step Given codelet/napi/src/work_units_watcher.rs after RPC-017 lands
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

/// Scenario: Existing TS prioritize-work-unit code path is untouched
#[test]
fn existing_ts_prioritize_work_unit_code_path_is_untouched() {
    // @step Given the project root after RPC-017 lands
    let codelet_root = workspace_root();
    let project_root = codelet_root
        .parent()
        .expect("workspace root must have a parent (project root)");
    let prio_ts = project_root
        .join("src")
        .join("commands")
        .join("prioritize-work-unit.ts");

    // @step Then the file src/commands/prioritize-work-unit.ts exists
    assert!(
        prio_ts.exists(),
        "src/commands/prioritize-work-unit.ts must exist (RPC-017 must not move/delete it)"
    );

    let prio_body = std::fs::read_to_string(&prio_ts)
        .unwrap_or_else(|e| panic!("read {}: {e}", prio_ts.display()));
    // @step And src/commands/prioritize-work-unit.ts still exports prioritizeWorkUnit and routes writes through fileManager.transaction
    // The TS file still references `prioritizeWorkUnit` and `fileManager.transaction`
    // — sentinel substrings that prove it has not been gutted.
    assert!(
        prio_body.contains("prioritizeWorkUnit"),
        "prioritize-work-unit.ts must still export prioritizeWorkUnit"
    );
    assert!(
        prio_body.contains("fileManager.transaction"),
        "prioritize-work-unit.ts must still use fileManager.transaction (TS path unchanged)"
    );

    // @step And src/tui/components/BoardView.tsx still exists at its pre-RPC-017 path
    let board_view = project_root
        .join("src")
        .join("tui")
        .join("components")
        .join("BoardView.tsx");
    assert!(board_view.exists(), "BoardView.tsx must still exist");

    // @step And src/tui/components/UnifiedBoardLayout.tsx still exists at its pre-RPC-017 path
    let unified = project_root
        .join("src")
        .join("tui")
        .join("components")
        .join("UnifiedBoardLayout.tsx");
    assert!(unified.exists(), "UnifiedBoardLayout.tsx must still exist");
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc
#[test]
fn views_do_not_directly_import_codelet_core_napi_tarpc() {
    // @step Given the directory codelet/fspec-tui/src/views/ after RPC-017 lands
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
