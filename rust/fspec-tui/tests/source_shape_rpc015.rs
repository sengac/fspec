//! RPC-015 — Source-shape regression for the BoardView header port +
//! shared CheckpointCounts type + new RPC method.
//!
//! Feature: spec/features/rpc015-source-shape.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn rpc_types_lib() -> std::path::PathBuf {
    common::workspace_root()
        .join("rpc-types")
        .join("src")
        .join("lib.rs")
}

fn rpc_lib() -> std::path::PathBuf {
    common::workspace_root()
        .join("rpc")
        .join("src")
        .join("lib.rs")
}

fn napi_git_rs() -> std::path::PathBuf {
    common::workspace_root()
        .join("napi")
        .join("src")
        .join("git.rs")
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

/// Scenario: Header widget modules exist as separate files
#[test]
fn header_widget_modules_exist_as_separate_files() {
    // @step Given the rust/fspec-tui crate after RPC-015 lands
    // @step When a developer scans the views/board/ directory
    // @step Then the file rust/fspec-tui/src/views/board/logo.rs exists
    let logo = src_dir().join("views").join("board").join("logo.rs");
    assert!(
        logo.exists(),
        "views/board/logo.rs must exist after RPC-015"
    );
    // @step And the file rust/fspec-tui/src/views/board/checkpoint_status.rs exists
    let cs = src_dir()
        .join("views")
        .join("board")
        .join("checkpoint_status.rs");
    assert!(
        cs.exists(),
        "views/board/checkpoint_status.rs must exist after RPC-015"
    );
    // @step And the file rust/fspec-tui/src/views/board/keybinding_shortcuts.rs exists
    let kb = src_dir()
        .join("views")
        .join("board")
        .join("keybinding_shortcuts.rs");
    assert!(
        kb.exists(),
        "views/board/keybinding_shortcuts.rs must exist after RPC-015"
    );
}

/// Scenario: New and modified board modules stay under 300 lines
#[test]
fn new_and_modified_board_modules_stay_under_300_lines() {
    // @step Given the directory rust/fspec-tui/src/views/board/ plus the views/board.rs orchestrator
    let board_dir = src_dir().join("views").join("board");
    let board_orchestrator = src_dir().join("views").join("board.rs");
    // @step When a test counts the line-count of every .rs file in views/board/ plus views/board.rs
    let mut targets: Vec<std::path::PathBuf> = vec![board_orchestrator];
    let entries = std::fs::read_dir(&board_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", board_dir.display()));
    for entry in entries {
        let entry = entry.expect("read_dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            targets.push(path);
        }
    }
    targets.sort();
    let names: Vec<String> = targets
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    for required in ["logo.rs", "checkpoint_status.rs", "keybinding_shortcuts.rs"] {
        assert!(
            names.iter().any(|n| n == required),
            "views/board/ must contain {required}; found: {names:?}"
        );
    }
    let mut violations = Vec::new();
    for path in &targets {
        let lines = count_lines_path(path);
        if lines >= 300 {
            violations.push(format!(
                "{}: {} lines >= 300 ceiling",
                path.display(),
                lines
            ));
        }
    }
    // @step Then views/board.rs has fewer than 300 lines
    // @step And views/board/logo.rs has fewer than 300 lines
    // @step And views/board/checkpoint_status.rs has fewer than 300 lines
    // @step And views/board/keybinding_shortcuts.rs has fewer than 300 lines
    assert!(
        violations.is_empty(),
        "RPC-015 board modules MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: CheckpointCounts shared type lives in rpc-types
#[test]
fn checkpoint_counts_shared_type_lives_in_rpc_types() {
    // @step Given rust/rpc-types/src/lib.rs after RPC-015 lands
    let body = read_raw(&rpc_types_lib());
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "pub struct CheckpointCounts"
    assert!(
        body.contains("pub struct CheckpointCounts"),
        "rpc-types must define `pub struct CheckpointCounts`"
    );
    // @step And the file contains the substring "pub manual: u32"
    assert!(
        body.contains("pub manual: u32"),
        "CheckpointCounts must have `pub manual: u32`"
    );
    // @step And the file contains the substring "pub auto: u32"
    assert!(
        body.contains("pub auto: u32"),
        "CheckpointCounts must have `pub auto: u32`"
    );
}

/// Scenario: FspecService trait gains the checkpoint_counts RPC method
#[test]
fn fspec_service_trait_gains_the_checkpoint_counts_rpc_method() {
    // @step Given rust/rpc/src/lib.rs after RPC-015 lands
    let body = read_raw(&rpc_lib());
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "async fn checkpoint_counts() -> CheckpointCounts"
    assert!(
        body.contains("async fn checkpoint_counts() -> CheckpointCounts"),
        "FspecService must declare `async fn checkpoint_counts() -> CheckpointCounts`"
    );
}

/// Scenario: FspecBackend trait gains the checkpoint_counts method
#[test]
fn fspec_backend_trait_gains_the_checkpoint_counts_method() {
    // @step Given rust/fspec-tui/src/transport/mod.rs after RPC-015 lands
    let body = read_raw(&src_dir().join("transport").join("mod.rs"));
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "async fn checkpoint_counts"
    assert!(
        body.contains("async fn checkpoint_counts"),
        "transport/mod.rs must declare `async fn checkpoint_counts` on FspecBackend"
    );
    // @step And the file contains the substring "CheckpointCounts"
    assert!(
        body.contains("CheckpointCounts"),
        "transport/mod.rs must reference the CheckpointCounts type"
    );
}

/// Scenario: Action enum gains CheckpointCountsLoaded variant
#[test]
fn action_enum_gains_checkpointcountsloaded_variant() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-015 lands
    let body = read_raw(&src_dir().join("components").join("mod.rs"));
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "CheckpointCountsLoaded"
    assert!(
        body.contains("CheckpointCountsLoaded"),
        "Action enum must include the `CheckpointCountsLoaded` variant"
    );
}

/// Scenario: NAPI surface exposes the additive count_checkpoints export
#[test]
fn napi_surface_exposes_the_additive_count_checkpoints_export() {
    // @step Given rust/napi/src/git.rs after RPC-015 lands
    let body = read_raw(&napi_git_rs());
    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "pub fn count_checkpoints"
    assert!(
        body.contains("pub fn count_checkpoints"),
        "rust/napi/src/git.rs must export `pub fn count_checkpoints`"
    );
    // @step And the file contains the substring "codelet_git::ghost_commit::count_checkpoints"
    assert!(
        body.contains("codelet_git::ghost_commit::count_checkpoints"),
        "napi count_checkpoints must delegate to codelet_git::ghost_commit::count_checkpoints"
    );
}

/// Scenario: RPC-013 / RPC-014 invariants preserved
#[test]
fn rpc013_and_rpc014_invariants_preserved() {
    // @step Given rust/fspec-tui/src/views/navigator.rs after RPC-015 lands
    let nav = read_stripped("views/navigator.rs");
    // @step Then the file does NOT contain "Constraint::Length(1)"
    assert!(
        !nav.contains("Constraint::Length(1)"),
        "navigator.rs must not re-introduce Constraint::Length(1) after RPC-013/RPC-014/RPC-015"
    );
    // @step And rust/fspec-tui/src/views/mod.rs does NOT contain the identifier "FooterView"
    let mod_rs = read_stripped("views/mod.rs");
    assert!(
        !mod_rs.contains("FooterView"),
        "views/mod.rs must not reference FooterView"
    );
    // @step And rust/fspec-tui/src/lib.rs does NOT contain the identifier "FooterView"
    let lib_rs = read_stripped("lib.rs");
    assert!(
        !lib_rs.contains("FooterView"),
        "lib.rs must not reference FooterView"
    );
    // @step And the file rust/fspec-tui/src/views/footer.rs does NOT exist
    let footer = src_dir().join("views").join("footer.rs");
    assert!(!footer.exists(), "views/footer.rs must not exist");
    // @step And rust/fspec-tui/src/views/board.rs still contains the substring "Action::EnterWorkUnit"
    let board = read_raw(&src_dir().join("views").join("board.rs"));
    assert!(
        board.contains("Action::EnterWorkUnit"),
        "board.rs must still emit Action::EnterWorkUnit"
    );
    // @step And rust/fspec-tui/src/views/board.rs still contains the substring "Action::FocusNextColumn"
    assert!(
        board.contains("Action::FocusNextColumn"),
        "board.rs must still emit Action::FocusNextColumn"
    );
    // @step And rust/fspec-tui/src/views/board.rs still contains the substring "Action::ReorderUp"
    assert!(
        board.contains("Action::ReorderUp"),
        "board.rs must still emit Action::ReorderUp"
    );
}

/// Scenario: Views still avoid encapsulated transport crates and host runtime construction
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() {
    // @step Given the directory rust/fspec-tui/src/views/ (including views/board/)
    let views_dir = src_dir().join("views");
    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        // @step Then NO file imports `codelet_napi::` or `codelet_core::` or `tarpc::` or `tokio_tungstenite::`
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
        // @step And NO file contains `tokio::runtime::Builder` or `Runtime::new()`
        for needle in ["tokio::runtime::Builder", "Runtime::new()"] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-015 must preserve transport-encapsulation + host-runtime invariants. Violations: {violations:?}"
    );
}
